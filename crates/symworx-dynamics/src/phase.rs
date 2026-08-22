// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Relative phase, Kuramoto order, and cluster-phase synchrony.
//!
//! Input is **already-extracted** phases in radians. This module does not
//! bandpass or apply a Hilbert transform — callers that have oscillatory
//! series should use `symworx-signal::AnalyticSignal` first.
//!
//! Cluster-phase follows Richardson, Garcia, Frank, Gergor & Marsh (2012).
//! `ρ_k` is lock *strength* (high at 0° or 180°). In-phase vs anti-phase is
//! the mean relative phase, not `ρ_k`.

use std::f64::consts::PI;

use symworx_math::circular::{
    angular_diff,
    circular_mean,
    circular_sd,
    mean_resultant_length,
    wrap_pi,
};

/// How a relative-phase distribution sits on the circle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseRelation {
    /// Circular mean near 0.
    InPhase,
    /// Circular mean near ±π.
    AntiPhase,
    /// Any other stable (or unstable) offset.
    Other,
}

/// Why a phase calculation could not run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhaseError {
    /// Series lengths differ or a required slice is empty.
    LengthMismatch,
    /// A sample was non-finite.
    NonFinite,
}

/// Pairwise relative-phase summary (strength and relation stay separate).
#[derive(Clone, Debug, PartialEq)]
pub struct RelativePhaseSummary {
    /// Circular mean of `Δφ` (≈0 in-phase, ≈π anti-phase).
    pub circular_mean: f64,
    /// Circular standard deviation of `Δφ`.
    pub circular_sd: f64,
    /// Mean resultant length of `Δφ` (lock strength, `[0, 1]`).
    pub mean_resultant_length: f64,
    /// Classification of [`Self::circular_mean`] against `in_phase_tol`.
    pub relation: PhaseRelation,
}

/// Cluster-phase result for `N` series (Richardson et al. 2012).
#[derive(Clone, Debug, PartialEq)]
pub struct ClusterPhaseResult {
    /// Group lock `(1/N) Σ ρ_k`, in `[0, 1]`.
    pub rho_group: f64,
    /// Per-series lock to the cluster phase.
    pub rho_k: Vec<f64>,
    /// Mean relative phase of each series to the cluster (`φ̄_k`).
    pub mean_rel_phase_k: Vec<f64>,
    /// Circular SD of each series' relative phase to the cluster.
    pub rel_phase_sd_k: Vec<f64>,
    /// Cluster phase `Φ(t)`.
    pub cluster_phase: Vec<f64>,
    /// Instantaneous Kuramoto order `|q̄(t)|`.
    pub kuramoto_r: Vec<f64>,
}

fn all_finite(xs: &[f64]) -> bool {
    xs.iter().all(|x| x.is_finite())
}

fn classify_mean(mean: f64, tol: f64) -> PhaseRelation {
    let m = wrap_pi(mean);
    if m.abs() <= tol {
        PhaseRelation::InPhase
    } else if (m.abs() - PI).abs() <= tol {
        PhaseRelation::AntiPhase
    } else {
        PhaseRelation::Other
    }
}

/// `wrap(φ_i − φ_j)` at each sample.
pub fn relative_phase(phi_i: &[f64], phi_j: &[f64]) -> Result<Vec<f64>, PhaseError> {
    if phi_i.len() != phi_j.len() || phi_i.is_empty() {
        return Err(PhaseError::LengthMismatch);
    }
    if !all_finite(phi_i) || !all_finite(phi_j) {
        return Err(PhaseError::NonFinite);
    }
    Ok(phi_i
        .iter()
        .zip(phi_j.iter())
        .map(|(&a, &b)| angular_diff(a, b))
        .collect())
}

/// Summarize a relative-phase series. `in_phase_tol` is radians (default callers: `π/4`).
pub fn summarize_relative_phase(delta: &[f64], in_phase_tol: f64) -> Option<RelativePhaseSummary> {
    if delta.is_empty() || !all_finite(delta) || !in_phase_tol.is_finite() || in_phase_tol < 0.0 {
        return None;
    }
    let mean = circular_mean(delta)?;
    let sd = circular_sd(delta)?;
    let r = mean_resultant_length(delta)?;
    Some(RelativePhaseSummary {
        circular_mean: mean,
        circular_sd: sd,
        mean_resultant_length: r,
        relation: classify_mean(mean, in_phase_tol),
    })
}

/// Instantaneous Kuramoto order parameter `R(t) ∈ [0, 1]`.
pub fn kuramoto_order(phases_at_t: &[f64]) -> Option<f64> {
    mean_resultant_length(phases_at_t)
}

/// Kuramoto `R` over time. Each inner slice is one series `φ_k(t)`.
pub fn kuramoto_order_series(phases: &[&[f64]]) -> Result<Vec<f64>, PhaseError> {
    let (_n, t_len) = series_shape(phases)?;
    Ok((0..t_len)
        .map(|t| {
            let at_t: Vec<f64> = phases.iter().map(|p| p[t]).collect();
            kuramoto_order(&at_t).unwrap_or(0.0)
        })
        .collect())
}

fn series_shape(phases: &[&[f64]]) -> Result<(usize, usize), PhaseError> {
    if phases.is_empty() {
        return Err(PhaseError::LengthMismatch);
    }
    let t_len = phases[0].len();
    if t_len == 0 {
        return Err(PhaseError::LengthMismatch);
    }
    for s in phases {
        if s.len() != t_len {
            return Err(PhaseError::LengthMismatch);
        }
        if !all_finite(s) {
            return Err(PhaseError::NonFinite);
        }
    }
    Ok((phases.len(), t_len))
}

/// Cluster-phase method (Richardson et al., 2012).
///
/// `phases[k][t]` is the instantaneous phase of series `k` at sample `t`.
///
/// # Example
/// ```
/// use symworx_dynamics::cluster_phase;
///
/// let a: Vec<f64> = (0..40).map(|i| 0.2 * i as f64).collect();
/// let r = cluster_phase(&[&a, &a, &a]).unwrap();
/// assert!((r.rho_group - 1.0).abs() < 1e-9);
/// ```
pub fn cluster_phase(phases: &[&[f64]]) -> Result<ClusterPhaseResult, PhaseError> {
    let (n, t_len) = series_shape(phases)?;
    let nf = n as f64;
    let tf = t_len as f64;

    let mut cluster_phase = Vec::with_capacity(t_len);
    let mut kuramoto_r = Vec::with_capacity(t_len);

    for t in 0..t_len {
        let mut c = 0.0;
        let mut s = 0.0;
        for series in phases {
            c += series[t].cos();
            s += series[t].sin();
        }
        c /= nf;
        s /= nf;
        kuramoto_r.push((c.hypot(s)).clamp(0.0, 1.0));
        cluster_phase.push(s.atan2(c));
    }

    let mut mean_rel_phase_k = Vec::with_capacity(n);
    let mut rel_phase_sd_k = Vec::with_capacity(n);
    let mut rho_k = Vec::with_capacity(n);

    for series in phases {
        let rel_buf: Vec<f64> = series
            .iter()
            .zip(cluster_phase.iter())
            .map(|(&phi, &phi_c)| angular_diff(phi, phi_c))
            .collect();
        let mean_rel = circular_mean(&rel_buf).unwrap_or(0.0);
        mean_rel_phase_k.push(mean_rel);
        rel_phase_sd_k.push(circular_sd(&rel_buf).unwrap_or(0.0));

        // θ_k(t) = φ̃_k(t) − φ̄_k ; ρ_k = |mean exp(i θ_k)|
        let mut c = 0.0;
        let mut s = 0.0;
        for rel in &rel_buf {
            let theta = angular_diff(*rel, mean_rel);
            c += theta.cos();
            s += theta.sin();
        }
        rho_k.push(((c / tf).hypot(s / tf)).clamp(0.0, 1.0));
    }

    let rho_group = rho_k.iter().sum::<f64>() / nf;

    Ok(ClusterPhaseResult {
        rho_group,
        rho_k,
        mean_rel_phase_k,
        rel_phase_sd_k,
        cluster_phase,
        kuramoto_r,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn almost(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn ramp(n: usize, offset: f64) -> Vec<f64> {
        (0..n).map(|i| offset + 0.2 * i as f64).collect()
    }

    #[test]
    fn relative_phase_in_and_anti() {
        let a = ramp(40, 0.0);
        let same = relative_phase(&a, &a).unwrap();
        let sum = summarize_relative_phase(&same, PI / 4.0).unwrap();
        assert_eq!(sum.relation, PhaseRelation::InPhase);
        assert!(almost(sum.mean_resultant_length, 1.0));

        let b: Vec<f64> = a.iter().map(|x| x + PI).collect();
        let anti = relative_phase(&a, &b).unwrap();
        let sum = summarize_relative_phase(&anti, PI / 4.0).unwrap();
        assert_eq!(sum.relation, PhaseRelation::AntiPhase);
        assert!(almost(sum.mean_resultant_length, 1.0));
    }

    #[test]
    fn identical_series_full_cluster_lock() {
        let p = ramp(50, 0.3);
        let refs: Vec<&[f64]> = vec![&p, &p, &p];
        let r = cluster_phase(&refs).unwrap();
        assert!((r.rho_group - 1.0).abs() < 1e-9);
        assert!(r.rho_k.iter().all(|x| (*x - 1.0).abs() < 1e-9));
        assert!(r.mean_rel_phase_k.iter().all(|x| x.abs() < 1e-9));
        assert!(r.kuramoto_r.iter().all(|x| (*x - 1.0).abs() < 1e-9));
    }

    #[test]
    fn two_groups_offset_by_pi() {
        let a = ramp(60, 0.0);
        let b: Vec<f64> = a.iter().map(|x| x + PI).collect();
        // Unequal sizes so Φ is defined (a balanced 2+2 split makes |q̄|=0).
        let refs: Vec<&[f64]> = vec![&a, &a, &a, &b, &b];
        let r = cluster_phase(&refs).unwrap();
        assert!(r.rho_k.iter().all(|x| *x > 0.99));
        let in_phase = r.mean_rel_phase_k.iter().filter(|m| m.abs() < 0.2).count();
        let anti = r.mean_rel_phase_k.iter().filter(|m| (m.abs() - PI).abs() < 0.2).count();
        assert_eq!(in_phase, 3);
        assert_eq!(anti, 2);
        let r_all = kuramoto_order_series(&refs).unwrap();
        let mean_r: f64 = r_all.iter().sum::<f64>() / r_all.len() as f64;
        assert!(mean_r > 0.15 && mean_r < 0.3, "expected |3−2|/5, got {mean_r}");
    }

    #[test]
    fn one_antiphase_member_still_locked() {
        let a = ramp(40, 0.0);
        let b: Vec<f64> = a.iter().map(|x| x + PI).collect();
        let refs: Vec<&[f64]> = vec![&a, &a, &a, &b];
        let r = cluster_phase(&refs).unwrap();
        assert!(r.rho_k[3] > 0.99);
        assert!((r.mean_rel_phase_k[3].abs() - PI).abs() < 0.2);
    }

    #[test]
    fn bad_input_errors() {
        assert_eq!(relative_phase(&[0.0], &[0.0, 1.0]), Err(PhaseError::LengthMismatch));
        assert_eq!(relative_phase(&[0.0], &[f64::NAN]), Err(PhaseError::NonFinite));
        assert!(cluster_phase(&[]).is_err());
        assert!(kuramoto_order(&[]).is_none());
    }
}
