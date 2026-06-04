// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use crate::physiology::common::local_maxima_indices;

/// Inhalation and exhalation peak indices partitioned by flow sign.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RespPhasePeaks {
    pub inhalation_peak_indices: Vec<usize>,
    pub exhalation_peak_indices: Vec<usize>,
}

/// Classify local maxima on a flow trace into inhalation (>0) and exhalation (<=0) peaks.
pub fn phase_peak_indices(flow: &[f64]) -> RespPhasePeaks {
    let mut inhalation_peak_indices = Vec::new();
    let mut exhalation_peak_indices = Vec::new();

    for i in local_maxima_indices(flow) {
        if flow[i] > 0.0 {
            inhalation_peak_indices.push(i);
        } else {
            exhalation_peak_indices.push(i);
        }
    }

    RespPhasePeaks {
        inhalation_peak_indices,
        exhalation_peak_indices,
    }
}

/// Inter-peak intervals (seconds) between consecutive indices of the same phase.
pub fn phase_peak_intervals_sec(indices: &[usize], fs: f64) -> Vec<f64> {
    if indices.len() < 2 || fs <= 0.0 {
        return vec![];
    }
    indices
        .windows(2)
        .map(|w| (w[1] - w[0]) as f64 / fs)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_peaks_partition_by_sign() {
        // Inhalation: positive local max; exhalation: negative local max (less negative crest).
        let flow = vec![0.0, 1.0, 0.0, -1.0, -0.5, -1.0, 0.0];
        let phases = phase_peak_indices(&flow);
        assert_eq!(phases.inhalation_peak_indices, vec![1]);
        assert_eq!(phases.exhalation_peak_indices, vec![4]);
    }
}
