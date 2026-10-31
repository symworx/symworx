// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

/// Recurrence quantification over a packed mask.
#[derive(Debug, Clone, Copy)]
pub struct RqaParams {
    /// Minimum run length counted as a diagonal or vertical line.
    pub min_line: usize,
}

impl Default for RqaParams {
    fn default() -> Self {
        Self { min_line: 2 }
    }
}

/// Summary statistics from one recurrence mask.
#[derive(Debug, Clone, PartialEq)]
pub struct RqaStats {
    /// Fraction of recurrent pairs.
    pub rr: f64,
    /// Fraction of recurrent points in diagonal lines of length >= min_line.
    pub det: f64,
    /// Fraction of recurrent points in vertical lines of length >= min_line.
    pub lam: f64,
    /// Mean diagonal line length.
    pub mean_diag: f64,
    /// Entropy of the diagonal line length histogram, nats.
    pub diag_entropy: f64,
}

/// Compute RQA from a recurrence mask.
pub fn rqa(mask: &crate::RecurrenceMask, params: RqaParams) -> RqaStats {
    let n = mask.nrrows;
    if n == 0 || mask.nccols == 0 {
        return RqaStats {
            rr: 0.0,
            det: 0.0,
            lam: 0.0,
            mean_diag: 0.0,
            diag_entropy: 0.0,
        };
    }

    let total = n * mask.nccols;
    let mut recurrent = 0usize;
    let mut diag_hist = alloc::vec::Vec::new();
    let mut vert_hist = alloc::vec::Vec::new();

    // Diagonal lines: constant i - j.
    for diff in -(n as isize - 1)..=(mask.nccols as isize - 1) {
        let mut len = 0usize;
        let mut count = 0usize;
        let (i0, j0) = if diff >= 0 {
            (0, diff as usize)
        } else {
            ((-diff) as usize, 0)
        };
        let mut i = i0;
        let mut j = j0;
        while i < n && j < mask.nccols {
            if mask.bit(i, j) {
                len += 1;
                recurrent += 1;
            } else if len > 0 {
                count_line(len, params.min_line, &mut diag_hist, &mut count);
                len = 0;
            }
            i += 1;
            j += 1;
        }
        if len > 0 {
            count_line(len, params.min_line, &mut diag_hist, &mut count);
        }
        let _ = count;
    }

    // Vertical lines: constant i, varying j.
    for i in 0..n {
        let mut len = 0usize;
        for j in 0..mask.nccols {
            if mask.bit(i, j) {
                len += 1;
            } else if len > 0 {
                count_line(len, params.min_line, &mut vert_hist, &mut 0usize);
                len = 0;
            }
        }
        if len > 0 {
            count_line(len, params.min_line, &mut vert_hist, &mut 0usize);
        }
    }

    let rr = recurrent as f64 / total as f64;

    let (det, mean_diag, diag_entropy) = line_stats(&diag_hist, recurrent);
    let (lam, _, _) = line_stats(&vert_hist, recurrent);

    RqaStats {
        rr,
        det,
        lam,
        mean_diag,
        diag_entropy,
    }
}

fn count_line(len: usize, min_line: usize, hist: &mut alloc::vec::Vec<usize>, _count: &mut usize) {
    if len >= min_line {
        if hist.len() <= len {
            hist.resize(len + 1, 0);
        }
        hist[len] += len;
    }
}

fn line_stats(hist: &[usize], recurrent: usize) -> (f64, f64, f64) {
    if recurrent == 0 {
        return (0.0, 0.0, 0.0);
    }
    let mut line_points = 0usize;
    let mut line_count = 0usize;
    let mut entropy = 0.0;
    for (len, &points) in hist.iter().enumerate() {
        if len == 0 || points == 0 {
            continue;
        }
        line_points += points;
        line_count += points / len;
        let p = points as f64 / recurrent as f64;
        entropy -= p * p.ln();
    }
    let det = line_points as f64 / recurrent as f64;
    let mean_diag = if line_count > 0 {
        line_points as f64 / line_count as f64
    } else {
        0.0
    };
    (det, mean_diag, entropy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recurrence::{
        Trajectory,
        recurrence_mask,
    };

    #[test]
    fn periodic_signal_has_high_determinism() {
        let points: alloc::vec::Vec<f64> = (0..64).map(|i| (i as f64 * 0.1).sin()).collect();
        let mask = recurrence_mask(
            Trajectory {
                points: &points,
                n: 64,
                dim: 1,
            },
            Trajectory {
                points: &points,
                n: 64,
                dim: 1,
            },
            0.3,
            1,
        )
        .unwrap();
        let stats = rqa(&mask, RqaParams::default());
        assert!(stats.rr > 0.0);
        assert!(stats.det > 0.5);
    }

    #[test]
    fn empty_mask_returns_zeros() {
        let mask = crate::RecurrenceMask {
            nrrows: 0,
            nccols: 0,
            bits: alloc::vec::Vec::new(),
        };
        let stats = rqa(&mask, RqaParams::default());
        assert_eq!(
            stats,
            RqaStats {
                rr: 0.0,
                det: 0.0,
                lam: 0.0,
                mean_diag: 0.0,
                diag_entropy: 0.0,
            }
        );
    }
}
