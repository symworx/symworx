// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Fixed-capacity ring buffers for live stream channels.

use std::collections::VecDeque;

use crate::{
    error::{
        EmbedError,
        Result,
    },
    types::StreamSample,
};

/// Rolling window of recent [`StreamSample`]s.
#[derive(Debug, Clone)]
pub struct SampleRing {
    cap: usize,
    buf: VecDeque<StreamSample>,
}

impl SampleRing {
    /// Create a ring with the given capacity (must be ≥ 1).
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 {
            return Err(EmbedError::InvalidParameter("ring capacity must be ≥ 1".into()));
        }
        Ok(Self {
            cap: capacity,
            buf: VecDeque::with_capacity(capacity),
        })
    }

    /// Push a sample, dropping the oldest if full.
    pub fn push(&mut self, sample: StreamSample) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(sample);
    }

    /// Number of samples currently stored.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Whether the ring is empty.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Configured capacity.
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Clear all samples.
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    /// Latest sample, if any.
    pub fn latest(&self) -> Option<&StreamSample> {
        self.buf.back()
    }

    /// Iterate oldest → newest.
    pub fn iter(&self) -> impl Iterator<Item = &StreamSample> {
        self.buf.iter()
    }

    /// Extract a channel as `f64` values (oldest → newest).
    ///
    /// Missing values become `f64::NAN` so series length matches the ring.
    pub fn channel_f64(&self, channel: Channel) -> Vec<f64> {
        self.buf
            .iter()
            .map(|s| {
                let v = match channel {
                    Channel::Red => s.red.map(|v| v as f64),
                    Channel::Ir => s.ir.map(|v| v as f64),
                    Channel::Bpm => s.bpm,
                    Channel::BpmAvg => s.bpm_avg,
                    Channel::Spo2 => s.spo2,
                    Channel::Resp => s.resp,
                };
                v.unwrap_or(f64::NAN)
            })
            .collect()
    }
}

/// Named numeric channel for [`SampleRing::channel_f64`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Red LED.
    Red,
    /// IR LED.
    Ir,
    /// Instantaneous BPM.
    Bpm,
    /// Averaged BPM.
    BpmAvg,
    /// SpO₂.
    Spo2,
    /// Respiration waveform sample.
    Resp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SourceKind;

    fn sample(ir: i64) -> StreamSample {
        StreamSample {
            ir: Some(ir),
            source: SourceKind::Simulator,
            ..Default::default()
        }
    }

    #[test]
    fn drops_oldest() {
        let mut r = SampleRing::new(3).unwrap();
        r.push(sample(1));
        r.push(sample(2));
        r.push(sample(3));
        r.push(sample(4));
        assert_eq!(r.len(), 3);
        let ir = r.channel_f64(Channel::Ir);
        assert_eq!(ir, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn zero_capacity_rejected() {
        assert!(SampleRing::new(0).is_err());
    }
}
