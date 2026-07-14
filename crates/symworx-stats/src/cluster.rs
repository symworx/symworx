// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Clustering algorithms.
//!
//! Foundational unsupervised methods for data-driven science and engineering
//! (Brunton & Kutz style exploratory analysis). Built on
//! [`crate::distance`] metrics — no LAPACK required for k-means.

use ndarray::Array2;

use crate::distance::euclidean;

/// Result of a k-means clustering run.
#[derive(Debug, Clone)]
pub struct KMeansResult {
    /// Cluster centroid coordinates (k × n_features).
    pub centroids: Array2<f64>,
    /// Hard assignment of each sample to a cluster index in `0..k`.
    pub labels: Vec<usize>,
    /// Within-cluster sum of squares (inertia).
    pub inertia: f64,
    /// Number of iterations until convergence (or `max_iter`).
    pub iterations: usize,
    /// `true` if assignments stabilized before `max_iter`.
    pub converged: bool,
}

/// Options for [`kmeans`].
#[derive(Debug, Clone)]
pub struct KMeansConfig {
    /// Number of clusters.
    pub k: usize,
    /// Maximum Lloyd iterations.
    pub max_iter: usize,
    /// Relative centroid movement tolerance for early stop.
    pub tol: f64,
    /// RNG seed for deterministic k-means++ initialization.
    pub seed: u64,
    /// If `true`, use k-means++ seeding; otherwise first `k` points (or wrap).
    pub kmeans_pp: bool,
}

impl Default for KMeansConfig {
    fn default() -> Self {
        Self {
            k: 2,
            max_iter: 100,
            tol: 1e-6,
            seed: 42,
            kmeans_pp: true,
        }
    }
}

/// K-means clustering (Lloyd's algorithm) with optional k-means++ init.
///
/// # Arguments
/// * `data` — samples as rows (n_samples × n_features)
/// * `config` — number of clusters, iterations, seed
///
/// Empty data or `k == 0` returns an empty result. If `k > n_samples`, `k` is
/// clamped to `n_samples`.
pub fn kmeans(data: &Array2<f64>, config: &KMeansConfig) -> KMeansResult {
    let n_samples = data.nrows();
    let n_features = data.ncols();

    if n_samples == 0 || n_features == 0 || config.k == 0 {
        return KMeansResult {
            centroids: Array2::zeros((0, n_features)),
            labels: Vec::new(),
            inertia: 0.0,
            iterations: 0,
            converged: true,
        };
    }

    let k = config.k.min(n_samples);
    let mut centroids = if config.kmeans_pp {
        kmeans_pp_init(data, k, config.seed)
    } else {
        naive_init(data, k)
    };

    let mut labels = vec![0usize; n_samples];
    let mut converged = false;
    let mut iterations = 0;

    for iter in 0..config.max_iter {
        iterations = iter + 1;

        // Assignment step
        for i in 0..n_samples {
            let row: Vec<f64> = data.row(i).to_vec();
            let mut best = 0usize;
            let mut best_d = f64::INFINITY;
            for j in 0..k {
                let c: Vec<f64> = centroids.row(j).to_vec();
                let d = euclidean(&row, &c);
                if d < best_d {
                    best_d = d;
                    best = j;
                }
            }
            labels[i] = best;
        }

        // Update step
        let mut new_centroids = Array2::<f64>::zeros((k, n_features));
        let mut counts = vec![0usize; k];
        for i in 0..n_samples {
            let lab = labels[i];
            counts[lab] += 1;
            for f in 0..n_features {
                new_centroids[[lab, f]] += data[[i, f]];
            }
        }
        for j in 0..k {
            if counts[j] > 0 {
                for f in 0..n_features {
                    new_centroids[[j, f]] /= counts[j] as f64;
                }
            } else {
                // Empty cluster: re-seed from a deterministic pseudo-random sample
                let mut seed = config
                    .seed
                    .wrapping_add((j as u64 + 1) * 0x9E37_79B9 + iterations as u64);
                let idx = (lcg_next(&mut seed) as usize) % n_samples;
                new_centroids.row_mut(j).assign(&data.row(idx));
            }
        }

        // Convergence: max centroid shift
        let mut max_shift = 0.0_f64;
        for j in 0..k {
            let old: Vec<f64> = centroids.row(j).to_vec();
            let new: Vec<f64> = new_centroids.row(j).to_vec();
            let shift = euclidean(&old, &new);
            if shift > max_shift {
                max_shift = shift;
            }
        }
        centroids = new_centroids;
        if max_shift < config.tol {
            converged = true;
            break;
        }
    }

    let inertia = compute_inertia(data, &centroids, &labels);

    KMeansResult {
        centroids,
        labels,
        inertia,
        iterations,
        converged,
    }
}

/// Within-cluster sum of squared Euclidean distances.
pub fn compute_inertia(data: &Array2<f64>, centroids: &Array2<f64>, labels: &[usize]) -> f64 {
    let mut inertia = 0.0;
    for (i, &lab) in labels.iter().enumerate() {
        let row: Vec<f64> = data.row(i).to_vec();
        let c: Vec<f64> = centroids.row(lab).to_vec();
        let d = euclidean(&row, &c);
        inertia += d * d;
    }
    inertia
}

/// Predict cluster labels for new points given fitted centroids.
pub fn kmeans_predict(data: &Array2<f64>, centroids: &Array2<f64>) -> Vec<usize> {
    let n_samples = data.nrows();
    let k = centroids.nrows();
    let mut labels = vec![0usize; n_samples];
    if k == 0 {
        return labels;
    }
    for i in 0..n_samples {
        let row: Vec<f64> = data.row(i).to_vec();
        let mut best = 0usize;
        let mut best_d = f64::INFINITY;
        for j in 0..k {
            let c: Vec<f64> = centroids.row(j).to_vec();
            let d = euclidean(&row, &c);
            if d < best_d {
                best_d = d;
                best = j;
            }
        }
        labels[i] = best;
    }
    labels
}

fn naive_init(data: &Array2<f64>, k: usize) -> Array2<f64> {
    let n_features = data.ncols();
    let mut centroids = Array2::zeros((k, n_features));
    for j in 0..k {
        centroids.row_mut(j).assign(&data.row(j % data.nrows()));
    }
    centroids
}

/// k-means++ initialization (Arthur & Vassilvitskii).
fn kmeans_pp_init(data: &Array2<f64>, k: usize, seed: u64) -> Array2<f64> {
    let n_samples = data.nrows();
    let n_features = data.ncols();
    let mut centroids = Array2::zeros((k, n_features));
    let mut rng = seed;

    // First center: uniform
    let first = (lcg_next(&mut rng) as usize) % n_samples;
    centroids.row_mut(0).assign(&data.row(first));

    let mut min_dist_sq = vec![f64::INFINITY; n_samples];

    for c in 1..k {
        // Update min squared distance to nearest chosen center
        for i in 0..n_samples {
            let row: Vec<f64> = data.row(i).to_vec();
            let center: Vec<f64> = centroids.row(c - 1).to_vec();
            let d = euclidean(&row, &center);
            let d2 = d * d;
            if d2 < min_dist_sq[i] {
                min_dist_sq[i] = d2;
            }
        }

        let total: f64 = min_dist_sq.iter().sum();
        let idx = if total < 1e-15 {
            (lcg_next(&mut rng) as usize) % n_samples
        } else {
            // Sample proportional to D(x)²
            let mut r = (lcg_next(&mut rng) as f64 / u64::MAX as f64) * total;
            let mut chosen = n_samples - 1;
            for (i, &d2) in min_dist_sq.iter().enumerate() {
                r -= d2;
                if r <= 0.0 {
                    chosen = i;
                    break;
                }
            }
            chosen
        };
        centroids.row_mut(c).assign(&data.row(idx));
    }

    centroids
}

/// Simple LCG for deterministic, dependency-free seeding.
fn lcg_next(state: &mut u64) -> u64 {
    // Numerical Recipes LCG
    *state = state
        .wrapping_mul(1664525)
        .wrapping_add(1013904223);
    *state
}

/// Column-wise mean of rows assigned to each label (utility for diagnostics).
pub fn cluster_sizes(labels: &[usize], k: usize) -> Vec<usize> {
    let mut sizes = vec![0usize; k];
    for &lab in labels {
        if lab < k {
            sizes[lab] += 1;
        }
    }
    sizes
}

// ——— Tests ———

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_kmeans_two_blobs() {
        // Two well-separated 2D clusters
        let data = array![
            [0.0, 0.0],
            [0.1, 0.1],
            [0.0, 0.2],
            [0.2, 0.0],
            [5.0, 5.0],
            [5.1, 5.0],
            [5.0, 5.2],
            [4.9, 5.1],
        ];

        let cfg = KMeansConfig {
            k: 2,
            max_iter: 50,
            tol: 1e-8,
            seed: 7,
            kmeans_pp: true,
        };
        let result = kmeans(&data, &cfg);

        assert_eq!(result.labels.len(), 8);
        assert_eq!(result.centroids.nrows(), 2);
        assert!(result.converged);

        // Points 0..3 should share a label; 4..7 the other
        let lab_a = result.labels[0];
        let lab_b = result.labels[4];
        assert_ne!(lab_a, lab_b);
        for i in 0..4 {
            assert_eq!(result.labels[i], lab_a);
        }
        for i in 4..8 {
            assert_eq!(result.labels[i], lab_b);
        }

        // Inertia should be small relative to between-cluster distance
        assert!(result.inertia < 1.0);
    }

    #[test]
    fn test_kmeans_predict() {
        let centroids = array![[0.0, 0.0], [10.0, 10.0]];
        let data = array![[0.1, 0.0], [9.0, 11.0], [1.0, 1.0]];
        let labels = kmeans_predict(&data, &centroids);
        assert_eq!(labels, vec![0, 1, 0]);
    }

    #[test]
    fn test_kmeans_empty() {
        let data = Array2::<f64>::zeros((0, 2));
        let result = kmeans(&data, &KMeansConfig::default());
        assert!(result.labels.is_empty());
        assert!(result.converged);
    }

    #[test]
    fn test_kmeans_k_clamped() {
        let data = array![[1.0], [2.0]];
        let cfg = KMeansConfig {
            k: 10,
            seed: 1,
            ..Default::default()
        };
        let result = kmeans(&data, &cfg);
        assert_eq!(result.centroids.nrows(), 2);
    }

    #[test]
    fn test_cluster_sizes() {
        let labels = vec![0, 0, 1, 1, 1];
        assert_eq!(cluster_sizes(&labels, 2), vec![2, 3]);
    }

    #[test]
    fn test_deterministic_seed() {
        let data = array![
            [0.0, 0.0],
            [1.0, 0.0],
            [0.0, 1.0],
            [10.0, 10.0],
            [11.0, 10.0],
            [10.0, 11.0],
        ];
        let cfg = KMeansConfig {
            k: 2,
            seed: 12345,
            ..Default::default()
        };
        let a = kmeans(&data, &cfg);
        let b = kmeans(&data, &cfg);
        assert_eq!(a.labels, b.labels);
        assert!((a.inertia - b.inertia).abs() < 1e-12);
    }
}
