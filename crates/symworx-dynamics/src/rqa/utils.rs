// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Internal utilities for RQA (line detection, Theiler window, etc.).

use std::collections::HashMap;

use ndarray::Array2;

/// Find lengths of all diagonal lines (parallel to main diagonal) with length ≥ min_length.
///
/// Because the input matrix already has Theiler-window cells forced to `false`,
/// lines are naturally terminated at the Theiler boundary. Both upper and lower
/// triangles are scanned.
pub(crate) fn find_diagonal_line_lengths(matrix: &Array2<bool>, min_length: usize) -> Vec<usize> {
    let n = matrix.nrows();
    if n == 0 || min_length == 0 {
        return Vec::new();
    }

    let mut lengths = Vec::new();

    // All diagonals: offset from -(n-1) to +(n-1)
    // offset > 0  =>  matrix[i, i+offset]
    // offset < 0  =>  matrix[i-offset, i]
    for offset in -(n as isize - 1)..=(n as isize - 1) {
        let mut current_len = 0usize;

        if offset >= 0 {
            let off = offset as usize;
            for i in 0..(n - off) {
                if matrix[[i, i + off]] {
                    current_len += 1;
                } else if current_len > 0 {
                    if current_len >= min_length {
                        lengths.push(current_len);
                    }
                    current_len = 0;
                }
            }
        } else {
            let off = (-offset) as usize;
            for i in 0..(n - off) {
                if matrix[[i + off, i]] {
                    current_len += 1;
                } else if current_len > 0 {
                    if current_len >= min_length {
                        lengths.push(current_len);
                    }
                    current_len = 0;
                }
            }
        }

        if current_len >= min_length {
            lengths.push(current_len);
        }
    }

    lengths
}

/// Find lengths of all vertical lines with length ≥ min_length.
///
/// Vertical lines indicate laminar (trapping) states.
pub(crate) fn find_vertical_line_lengths(matrix: &Array2<bool>, min_length: usize) -> Vec<usize> {
    let n = matrix.nrows();
    if n == 0 || min_length == 0 {
        return Vec::new();
    }

    let mut lengths = Vec::new();

    for col in 0..n {
        let mut current_len = 0usize;

        for row in 0..n {
            if matrix[[row, col]] {
                current_len += 1;
            } else if current_len > 0 {
                if current_len >= min_length {
                    lengths.push(current_len);
                }
                current_len = 0;
            }
        }

        if current_len >= min_length {
            lengths.push(current_len);
        }
    }

    lengths
}

/// Compute Shannon entropy (base 2) of the discrete distribution of line lengths.
///
/// `lengths` should already be filtered to those ≥ the chosen min_length.
pub(crate) fn line_length_entropy(lengths: &[usize]) -> f64 {
    if lengths.is_empty() {
        return 0.0;
    }

    let mut counts: HashMap<usize, usize> = HashMap::new();
    for &len in lengths {
        *counts.entry(len).or_insert(0) += 1;
    }

    let total = lengths.len() as f64;
    let mut entropy = 0.0;

    for &count in counts.values() {
        let p = count as f64 / total;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Count the total number of recurrent points (true cells) in the matrix.
pub(crate) fn count_recurrences(matrix: &Array2<bool>) -> usize {
    matrix.iter().filter(|&&b| b).count()
}

// TESTS
#[cfg(test)]
mod tests {
    use ndarray::Array2;

    use super::*;

    #[test]
    fn test_diagonal_line_detection_simple() {
        // 4x4 matrix with one clear diagonal line of length 3 on the main diagonal
        // (theiler would normally kill main diag, but here we test the scanner)
        let mut m = Array2::from_elem((4, 4), false);
        m[[0, 0]] = true;
        m[[1, 1]] = true;
        m[[2, 2]] = true;
        // broken at [3,3]

        let lengths = find_diagonal_line_lengths(&m, 2);
        assert!(lengths.contains(&3) || lengths.contains(&2));
    }

    #[test]
    fn test_vertical_line_detection() {
        let mut m = Array2::from_elem((5, 3), false);
        // Column 1 has a vertical run of 4
        m[[0, 1]] = true;
        m[[1, 1]] = true;
        m[[2, 1]] = true;
        m[[3, 1]] = true;

        let lengths = find_vertical_line_lengths(&m, 3);
        assert!(lengths.iter().any(|&l| l >= 4));
    }

    #[test]
    fn test_line_entropy_uniform() {
        let lengths = vec![2, 2, 2, 2];
        let e = line_length_entropy(&lengths);
        assert!((e - 0.0).abs() < 1e-12); // only one symbol
    }

    #[test]
    fn test_count_recurrences() {
        let mut m = Array2::from_elem((3, 3), false);
        m[[0, 1]] = true;
        m[[2, 2]] = true;
        assert_eq!(count_recurrences(&m), 2);
    }
}

// /// Find lengths of all vertical lines ≥ min_length
// fn find_vertical_line_lengths(matrix: &Array2<bool>, min_length: usize) -> Vec<usize> {
//     let n = matrix.nrows();
//     let mut lengths = Vec::new();

//     for col in 0..n {
//         let mut current_len = 0;
//         for row in 0..n {
//             if matrix[[row, col]] {
//                 current_len += 1;
//             } else {
//                 if current_len >= min_length {
//                     lengths.push(current_len);
//                 }
//                 current_len = 0;
//             }
//         }
//         if current_len >= min_length {
//             lengths.push(current_len);
//         }
//     }

//     lengths
// }

// fn compute_determinism_metrics(lengths: &[usize], n_recurrences: usize) -> (f64, usize, f64) {
//     if lengths.is_empty() || n_recurrences == 0 {
//         return (0.0, 0, 0.0);
//     }

//     let total_in_lines: usize = lengths.iter().sum();
//     let determinism = total_in_lines as f64 / n_recurrences as f64;

//     let lmax = *lengths.iter().max().unwrap_or(&0);

//     // Entropy of line length distribution
//     let lentr = compute_line_entropy(lengths);

//     (determinism, lmax, lentr)
// }

// fn compute_laminarity_metrics(lengths: &[usize], n_recurrences: usize) -> (f64, usize, f64) {
//     if lengths.is_empty() || n_recurrences == 0 {
//         return (0.0, 0, 0.0);
//     }

//     let total_in_lines: usize = lengths.iter().sum();
//     let laminarity = total_in_lines as f64 / n_recurrences as f64;

//     let vmax = *lengths.iter().max().unwrap_or(&0);

//     // Trapping time = average length of vertical lines
//     let trapping_time = if !lengths.is_empty() {
//         total_in_lines as f64 / lengths.len() as f64
//     } else {
//         0.0
//     };

//     (laminarity, vmax, trapping_time)
// }

// fn compute_line_entropy(lengths: &[usize]) -> f64 {
//     if lengths.is_empty() {
//         return 0.0;
//     }

//     use std::collections::HashMap;

//     let mut counts: HashMap<usize, usize> = HashMap::new();
//     for &len in lengths {
//         *counts.entry(len).or_insert(0) += 1;
//     }

//     let total = lengths.len() as f64;
//     let mut entropy = 0.0;

//     for &count in counts.values() {
//         let p = count as f64 / total;
//         if p > 0.0 {
//             entropy -= p * p.log2();
//         }
//     }

//     entropy
// }
