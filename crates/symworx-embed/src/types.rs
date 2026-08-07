// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Stream sample types and source metadata.
//!
//! SymWorx uses **subject** terminology (`sid`), not patient.

use std::time::SystemTime;

/// Where a sample originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceKind {
    /// Arduino / PlatformIO firmware over serial (or BLE bridge).
    Arduino,
    /// Host-side synthetic generator.
    Simulator,
    /// Native BLE host path (future).
    Ble,
    /// Unknown or unspecified source.
    #[default]
    Unknown,
}

impl SourceKind {
    /// Parse a short source tag (`arduino`, `simulator`, `ble`, …).
    pub fn from_tag(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "arduino" | "device" => Self::Arduino,
            "simulator" | "sim" => Self::Simulator,
            "ble" | "bluetooth" => Self::Ble,
            _ => Self::Unknown,
        }
    }

    /// Stable wire tag for outbound payloads.
    pub fn as_tag(self) -> &'static str {
        match self {
            Self::Arduino => "arduino",
            Self::Simulator => "simulator",
            Self::Ble => "ble",
            Self::Unknown => "unknown",
        }
    }
}

/// One multi-channel vitals / PPG sample from a stream.
///
/// Field names follow the device JSON line (`red`, `ir`, `bpm`, `bpm_avg`,
/// device `ts`) with host enrichment using **`sid`** (subject id).
#[derive(Debug, Clone, PartialEq)]
pub struct StreamSample {
    /// Red LED ADC count (MAX3010x).
    pub red: Option<i64>,
    /// IR LED ADC count (MAX3010x).
    pub ir: Option<i64>,
    /// Instantaneous heart rate (BPM), if available.
    pub bpm: Option<f64>,
    /// Short-window average BPM, if available.
    pub bpm_avg: Option<f64>,
    /// SpO₂ percent, if available.
    pub spo2: Option<f64>,
    /// Respiration / belt / impedance sample (normalized or raw), if available.
    pub resp: Option<f64>,
    /// Device timestamp in milliseconds (Arduino `millis()`).
    pub device_ts_ms: Option<u64>,
    /// Host receive / generate time.
    pub host_ts: Option<SystemTime>,
    /// Subject id (`sid` on the wire).
    pub sid: Option<String>,
    /// Provenance of this sample.
    pub source: SourceKind,
}

impl Default for StreamSample {
    fn default() -> Self {
        Self {
            red: None,
            ir: None,
            bpm: None,
            bpm_avg: None,
            spo2: None,
            resp: None,
            device_ts_ms: None,
            host_ts: None,
            sid: None,
            source: SourceKind::Unknown,
        }
    }
}

impl StreamSample {
    /// Best available heart-rate value (`bpm`, else `bpm_avg`).
    pub fn heart_rate(&self) -> Option<f64> {
        self.bpm.or(self.bpm_avg)
    }

    /// Attach subject id and optional host timestamp (default: now).
    pub fn with_sid(mut self, sid: impl Into<String>) -> Self {
        self.sid = Some(sid.into());
        if self.host_ts.is_none() {
            self.host_ts = Some(SystemTime::now());
        }
        self
    }

    /// Override source kind.
    pub fn with_source(mut self, source: SourceKind) -> Self {
        self.source = source;
        self
    }
}
