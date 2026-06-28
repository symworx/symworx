// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

use symworx_core::stats::{
    self,
    variability,
};

/// Heart rate variability metrics derived from RR intervals (seconds).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HrvMetrics {
    /// Root mean square of successive RR differences (seconds).
    pub rmssd_sec: Option<f64>,
    /// Standard deviation of RR intervals — SDNN (seconds).
    pub sdnn_sec: Option<f64>,
}

/// Compute RMSSD and SDNN from inter-beat intervals.
///
/// Requires at least three intervals for RMSSD and two for SDNN; otherwise fields are `None`.
pub fn compute_hrv_metrics(rr_intervals_sec: &[f64]) -> HrvMetrics {
    let rmssd_sec = if rr_intervals_sec.len() >= 3 {
        let v = variability::rmssd(rr_intervals_sec);
        if v.is_finite() { Some(v) } else { None }
    } else {
        None
    };

    let sdnn_sec = if rr_intervals_sec.len() >= 2 {
        let v = stats::std_dev(rr_intervals_sec);
        if v.is_finite() { Some(v) } else { None }
    } else {
        None
    };

    HrvMetrics {
        rmssd_sec,
        sdnn_sec,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hrv_from_constant_rr() {
        let rr = vec![0.8, 0.8, 0.8, 0.8];
        let hrv = compute_hrv_metrics(&rr);
        assert_eq!(hrv.rmssd_sec, Some(0.0));
        assert_eq!(hrv.sdnn_sec, Some(0.0));
    }

    #[test]
    fn hrv_insufficient_intervals() {
        let hrv = compute_hrv_metrics(&[0.9]);
        assert!(hrv.rmssd_sec.is_none());
        assert!(hrv.sdnn_sec.is_none());
    }
}
