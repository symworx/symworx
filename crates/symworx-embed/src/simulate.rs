// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Synthetic vitals stream (no hardware).
//!
//! Feature: `simulate` (enabled by default).
//!
//! Generates concurrent **PPG-like** (red/IR + BPM) and **respiration**
//! channels so the TUI can show a split live view.

use std::{
    thread,
    time::{
        Duration,
        Instant,
        SystemTime,
    },
};

use crate::{
    error::Result,
    source::StreamSource,
    types::{
        SourceKind,
        StreamSample,
    },
};

/// Configuration for [`SimulatorSource`].
#[derive(Debug, Clone)]
pub struct SimulatorConfig {
    /// Subject id attached to every sample (`sid`).
    pub sid: String,
    /// Target inter-sample delay.
    pub interval: Duration,
    /// Base BPM (oscillates slightly).
    pub bpm_base: f64,
    /// BPM amplitude of slow oscillation.
    pub bpm_amp: f64,
    /// Base SpO₂.
    pub spo2_base: f64,
    /// Whether to include synthetic red/IR PPG-like counts.
    pub include_ppg: bool,
    /// Whether to include a synthetic respiration waveform (`resp`).
    pub include_resp: bool,
    /// Base respiratory rate (breaths per minute).
    pub brpm_base: f64,
    /// Slow amplitude swing on BRPM.
    pub brpm_amp: f64,
    /// Optional max samples before EOF (`None` = infinite).
    pub max_samples: Option<u64>,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            sid: "S001".into(),
            interval: Duration::from_millis(100),
            bpm_base: 72.0,
            bpm_amp: 8.0,
            spo2_base: 97.0,
            include_ppg: true,
            include_resp: true,
            brpm_base: 14.0,
            brpm_amp: 2.0,
            max_samples: None,
        }
    }
}

/// Generates enriched vitals samples for demos and TUI development.
pub struct SimulatorSource {
    cfg: SimulatorConfig,
    started: Instant,
    n: u64,
    /// Integrated cardiac phase (radians) — must accumulate, not `ω(t)*t`.
    ppg_phase: f64,
    /// Integrated respiratory phase (radians).
    resp_phase: f64,
    /// Wall time of last sample (for phase steps).
    last_t: f64,
    /// Simple LCG for lightweight noise without external RNG crates.
    state: u64,
}

impl SimulatorSource {
    /// Create a simulator with the given config.
    pub fn new(cfg: SimulatorConfig) -> Self {
        Self {
            cfg,
            started: Instant::now(),
            n: 0,
            ppg_phase: 0.0,
            resp_phase: 0.0,
            last_t: 0.0,
            state: 0xC0FFEE_u64,
        }
    }

    /// Convenience: default config with custom `sid`.
    pub fn with_sid(sid: impl Into<String>) -> Self {
        let mut cfg = SimulatorConfig::default();
        cfg.sid = sid.into();
        Self::new(cfg)
    }

    fn next_u32(&mut self) -> u32 {
        // Numerical Recipes LCG
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.state >> 16) as u32
    }

    fn unit(&mut self) -> f64 {
        (self.next_u32() as f64) / (u32::MAX as f64)
    }
}

impl StreamSource for SimulatorSource {
    fn next_sample(&mut self) -> Result<Option<StreamSample>> {
        if let Some(max) = self.cfg.max_samples {
            if self.n >= max {
                return Ok(None);
            }
        }

        if self.n > 0 && !self.cfg.interval.is_zero() {
            thread::sleep(self.cfg.interval);
        }

        let t = self.started.elapsed().as_secs_f64();
        let dt = if self.n == 0 {
            0.0
        } else {
            (t - self.last_t).max(0.0)
        };
        self.last_t = t;

        // Slow HR / BR drift (labels). Keep label noise tiny so the waveform
        // stays clean; phase uses the smooth rate, not the jittered label.
        let slow = (t * 0.4).sin();
        let bpm_smooth = self.cfg.bpm_base + self.cfg.bpm_amp * slow;
        let bpm = bpm_smooth + (self.unit() - 0.5) * 0.4;
        let bpm_avg = self.cfg.bpm_base + self.cfg.bpm_amp * 0.5 * slow;
        let spo2 = (self.cfg.spo2_base + (self.unit() - 0.5) * 0.4).clamp(90.0, 100.0);

        let (red, ir) = if self.cfg.include_ppg {
            // Integrate phase: dφ/dt = 2π * f. Using sin(ω(t)*t) warps badly
            // as BPM drifts and made the live chart look "messy over time".
            let omega = std::f64::consts::TAU * (bpm_smooth / 60.0);
            self.ppg_phase += omega * dt;
            // Soft PPG-ish shape: fundamental + small 2nd harmonic (stable phase).
            let pulse = self.ppg_phase.sin() + 0.22 * (2.0 * self.ppg_phase).sin();
            // Very light sensor noise (was ±250 counts before → looked grainy).
            let noise = (self.unit() - 0.5) * 80.0;
            let ir = 80_000.0 + 15_000.0 * pulse + noise;
            let red = 100_000.0 + 10_000.0 * pulse + noise * 0.8;
            (Some(red as i64), Some(ir as i64))
        } else {
            (None, None)
        };

        let resp = if self.cfg.include_resp {
            let brpm = self.cfg.brpm_base + self.cfg.brpm_amp * (t * 0.15).sin();
            let omega_r = std::f64::consts::TAU * (brpm / 60.0);
            self.resp_phase += omega_r * dt;
            let breath = self.resp_phase.sin();
            Some(breath + 0.02 * (self.unit() - 0.5))
        } else {
            None
        };

        let sample = StreamSample {
            red,
            ir,
            bpm: Some(bpm),
            bpm_avg: Some(bpm_avg),
            spo2: Some(spo2),
            resp,
            device_ts_ms: Some(self.started.elapsed().as_millis() as u64),
            host_ts: Some(SystemTime::now()),
            sid: Some(self.cfg.sid.clone()),
            source: SourceKind::Simulator,
        };

        self.n += 1;
        Ok(Some(sample))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_stream_includes_ppg_and_resp() {
        let mut src = SimulatorSource::new(SimulatorConfig {
            interval: Duration::ZERO,
            max_samples: Some(5),
            ..Default::default()
        });
        let mut count = 0;
        while let Some(s) = src.next_sample().unwrap() {
            assert_eq!(s.sid.as_deref(), Some("S001"));
            assert_eq!(s.source, SourceKind::Simulator);
            assert!(s.ir.is_some());
            assert!(s.resp.is_some());
            count += 1;
        }
        assert_eq!(count, 5);
    }
}
