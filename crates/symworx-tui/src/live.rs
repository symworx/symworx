// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Live stream session for Explore (host-side; `symworx-embed`).
//!
//! Phase 1: **simulator** only (dual PPG + respiration). Serial can share
//! the same session shape later.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use symworx_embed::{
    analyze_vitals, Channel, SampleRing, SimulatorConfig, SimulatorSource, StreamSample,
    StreamSource, VitalsStatus,
};

/// Nominal sample period for the live simulator (UI ~10 Hz redraw).
const SAMPLE_INTERVAL_MS: u64 = 50;

/// Rolling window length in samples (oldest dropped when full).
/// At 50 ms → 20 Hz → **15 s** of visible history.
pub const LIVE_WINDOW_SAMPLES: usize = 300;

/// Active live ingest session (background producer + main-thread ring).
pub struct LiveSession {
    /// Subject id on the wire (`sid`).
    pub sid: String,
    /// Short label for UI (`simulator`, later `serial`).
    pub source_label: &'static str,
    /// Configured rolling window (ring capacity).
    pub window_samples: usize,
    /// Nominal samples/sec (for “window = Xs” labels).
    pub sample_hz: f64,
    stop: Arc<AtomicBool>,
    rx: Option<Receiver<StreamSample>>,
    join: Option<JoinHandle<()>>,
    ring: SampleRing,
    /// Total samples accepted on the host (may exceed window).
    pub samples_recv: u64,
    pub last_bpm: Option<f64>,
    pub last_bpm_avg: Option<f64>,
    pub last_spo2: Option<f64>,
    pub last_resp: Option<f64>,
    pub last_status: VitalsStatus,
}

impl LiveSession {
    /// Start a synthetic dual-channel stream (PPG + respiration).
    pub fn start_simulator(sid: impl Into<String>) -> Self {
        Self::start_simulator_window(sid, LIVE_WINDOW_SAMPLES)
    }

    /// Start simulator with an explicit rolling window length.
    pub fn start_simulator_window(sid: impl Into<String>, window_samples: usize) -> Self {
        let sid = sid.into();
        let window_samples = window_samples.max(16);
        let sample_hz = 1000.0 / SAMPLE_INTERVAL_MS as f64;
        let (tx, rx) = mpsc::sync_channel::<StreamSample>(64);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_t = Arc::clone(&stop);
        let sid_t = sid.clone();

        let join = thread::spawn(move || {
            let mut src = SimulatorSource::new(SimulatorConfig {
                sid: sid_t,
                interval: Duration::from_millis(SAMPLE_INTERVAL_MS),
                include_ppg: true,
                include_resp: true,
                max_samples: None,
                ..Default::default()
            });
            while !stop_t.load(Ordering::Relaxed) {
                match src.next_sample() {
                    Ok(Some(sample)) => match tx.try_send(sample) {
                        Ok(()) => {}
                        Err(mpsc::TrySendError::Full(s)) => {
                            if tx.send(s).is_err() {
                                break;
                            }
                        }
                        Err(mpsc::TrySendError::Disconnected(_)) => break,
                    },
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        });

        Self {
            sid,
            source_label: "simulator",
            window_samples,
            sample_hz,
            stop,
            rx: Some(rx),
            join: Some(join),
            ring: SampleRing::new(window_samples).expect("window >= 1"),
            samples_recv: 0,
            last_bpm: None,
            last_bpm_avg: None,
            last_spo2: None,
            last_resp: None,
            last_status: VitalsStatus::Normal,
        }
    }

    /// Wall-clock span of the rolling window (seconds).
    pub fn window_secs(&self) -> f64 {
        self.window_samples as f64 / self.sample_hz.max(1e-9)
    }

    /// Drain pending samples into the ring (call once per UI frame).
    ///
    /// When the ring is full, each push drops the oldest sample — that is the
    /// rolling window.
    pub fn poll(&mut self) {
        let Some(rx) = self.rx.as_ref() else {
            return;
        };
        loop {
            match rx.try_recv() {
                Ok(sample) => {
                    self.samples_recv = self.samples_recv.saturating_add(1);
                    self.last_bpm = sample.bpm;
                    self.last_bpm_avg = sample.bpm_avg;
                    self.last_spo2 = sample.spo2;
                    self.last_resp = sample.resp;
                    self.last_status = analyze_vitals(&sample);
                    self.ring.push(sample);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    /// PPG IR series (oldest → newest within the rolling window).
    pub fn ir_series(&self) -> Vec<f64> {
        self.ring.channel_f64(Channel::Ir)
    }

    /// Respiration series (oldest → newest within the rolling window).
    pub fn resp_series(&self) -> Vec<f64> {
        self.ring.channel_f64(Channel::Resp)
    }

    /// BPM series (oldest → newest).
    pub fn bpm_series(&self) -> Vec<f64> {
        self.ring.channel_f64(Channel::Bpm)
    }

    /// Samples currently held in the window (≤ capacity).
    pub fn ring_len(&self) -> usize {
        self.ring.len()
    }

    /// Whether the window is full and rolling (dropping oldest).
    pub fn is_rolling(&self) -> bool {
        self.ring.len() >= self.window_samples
    }

    /// Signal the worker to exit and join.
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.rx.take();
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for LiveSession {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.rx.take();
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}
