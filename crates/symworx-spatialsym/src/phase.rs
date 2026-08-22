// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Pairwise in-phase / out-of-phase effort and directional scoring.
//!
//! Effort-phase ignores heading (do they change speed together?).
//! Directional-phase keeps the four-cell table of effort × heading.
//! These are not collapsed into a single coherence number.

use symworx_math::circular::angular_diff;

use crate::{
    error::{
        Result,
        SpatialError,
    },
    geometry::Point2,
    kinematics::{
        EffortEvent,
        accel_decel_events,
        derive_headings,
        derive_scalar_accels,
        derive_speeds,
        derive_velocities_from_times,
    },
};

/// Threshold below which a signed acceleration is treated as no effort.
const ACCEL_EPS: f64 = 1e-12;

/// Window and gates for pairwise phase scoring.
#[derive(Clone, Debug, PartialEq)]
pub struct PhaseWindow {
    /// Analysis half-width (seconds) for [`pairwise_effort_phase_series`] / `*_at`.
    /// `0` means “full series” on those entry points. Session helpers ignore this
    /// and always score the whole bout.
    pub window_sec: f64,
    /// Max allowed clock gap (seconds) on the shared time base. `0` disables the check.
    pub max_gap_sec: f64,
    /// Maximum |lag| (seconds) when aligning the partner. `0` = contemporaneous only.
    pub lag_sec: f64,
    /// Acceleration event threshold (m/s²), same meaning as the count helper.
    pub accel_threshold: f64,
    /// `|Δheading|` at or below this (radians) counts as the same heading.
    pub heading_gate_rad: f64,
    /// Speeds at or below this (m/s) have no heading.
    pub min_speed: f64,
}

impl Default for PhaseWindow {
    fn default() -> Self {
        Self {
            window_sec: 0.0,
            max_gap_sec: 0.0,
            lag_sec: 0.0,
            accel_threshold: 0.8,
            heading_gate_rad: std::f64::consts::FRAC_PI_2,
            min_speed: 0.1,
        }
    }
}

/// Non-directional pairwise effort timing.
#[derive(Clone, Debug, PartialEq)]
pub struct PairwiseEffortPhase {
    /// Samples where both agents had an accel event or both had a decel event.
    pub in_phase_events: usize,
    /// Samples where one accelerated and the other decelerated.
    pub out_of_phase_events: usize,
    /// `in / (in + out)` when any events were comparable.
    pub event_in_phase_fraction: Option<f64>,
    /// Fraction of samples where `sign(a_i) == sign(a_j)` (both `|a|` above eps).
    pub sign_agree_fraction: Option<f64>,
    /// Lag applied to the second agent (seconds). Positive means `j` is later.
    pub lag_sec: f64,
}

/// Four-cell directional relation (effort sign × heading).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectionalRelation {
    /// Same effort sign, same heading.
    InPhase,
    /// Same effort sign, opposite heading.
    SpatiallyOpposed,
    /// Opposite effort sign, same heading.
    OutOfPhase,
    /// Opposite effort sign, opposite heading (treated as out-of-phase).
    MixedOutOfPhase,
}

impl DirectionalRelation {
    /// Compact label for summaries (`in` / `opp` / `out` / `mix`).
    pub fn short_label(self) -> &'static str {
        match self {
            Self::InPhase => "in",
            Self::SpatiallyOpposed => "opp",
            Self::OutOfPhase => "out",
            Self::MixedOutOfPhase => "mix",
        }
    }

    fn from_bools(same_effort: bool, same_heading: bool) -> Self {
        match (same_effort, same_heading) {
            (true, true) => Self::InPhase,
            (true, false) => Self::SpatiallyOpposed,
            (false, true) => Self::OutOfPhase,
            (false, false) => Self::MixedOutOfPhase,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::InPhase => 0,
            Self::SpatiallyOpposed => 1,
            Self::OutOfPhase => 2,
            Self::MixedOutOfPhase => 3,
        }
    }
}

/// Directional pairwise scores. Counts are `[InPhase, SpatiallyOpposed, OutOfPhase, MixedOutOfPhase]`.
#[derive(Clone, Debug, PartialEq)]
pub struct PairwiseDirectionalPhase {
    /// Sample counts for each [`DirectionalRelation`].
    pub counts: [usize; 4],
    /// Samples skipped because a heading was undefined (near-zero speed).
    pub heading_undefined: usize,
    /// Relation with the largest count, if any cell is non-zero.
    pub dominant: Option<DirectionalRelation>,
}

/// Signed closing / opening between two agents (asymmetric `i → j` vs `j → i`).
#[derive(Clone, Debug, PartialEq)]
pub struct PairwiseClosing {
    /// Mean `a_close` of `i` toward `j` (m/s²). Positive = closing.
    pub mean_i_toward_j: Option<f64>,
    /// Mean `a_close` of `j` toward `i` (m/s²).
    pub mean_j_toward_i: Option<f64>,
    /// Fraction of samples where both accelerate toward each other.
    pub both_closing_frac: Option<f64>,
    /// Fraction of samples where both accelerate away.
    pub both_opening_frac: Option<f64>,
    /// Fraction of samples with opposite closing signs.
    pub mixed_frac: Option<f64>,
}

impl PairwiseDirectionalPhase {
    fn from_counts(counts: [usize; 4], heading_undefined: usize) -> Self {
        let dominant = [0, 1, 2, 3]
            .into_iter()
            .max_by_key(|&i| counts[i])
            .filter(|&i| counts[i] > 0)
            .map(|i| match i {
                0 => DirectionalRelation::InPhase,
                1 => DirectionalRelation::SpatiallyOpposed,
                2 => DirectionalRelation::OutOfPhase,
                _ => DirectionalRelation::MixedOutOfPhase,
            });
        Self {
            counts,
            heading_undefined,
            dominant,
        }
    }
}

fn validate_cfg(cfg: &PhaseWindow) -> Result<()> {
    if !cfg.window_sec.is_finite() || cfg.window_sec < 0.0 {
        return Err(SpatialError::InvalidParameter("window_sec must be >= 0".into()));
    }
    if !cfg.max_gap_sec.is_finite() || cfg.max_gap_sec < 0.0 {
        return Err(SpatialError::InvalidParameter("max_gap_sec must be >= 0".into()));
    }
    if !cfg.lag_sec.is_finite() || cfg.lag_sec < 0.0 {
        return Err(SpatialError::InvalidParameter("lag_sec must be >= 0".into()));
    }
    if !cfg.accel_threshold.is_finite() || cfg.accel_threshold < 0.0 {
        return Err(SpatialError::InvalidParameter("accel_threshold must be >= 0".into()));
    }
    if !cfg.heading_gate_rad.is_finite() || cfg.heading_gate_rad < 0.0 {
        return Err(SpatialError::InvalidParameter("heading_gate_rad must be >= 0".into()));
    }
    if !cfg.min_speed.is_finite() || cfg.min_speed < 0.0 {
        return Err(SpatialError::InvalidParameter("min_speed must be >= 0".into()));
    }
    Ok(())
}

fn require_aligned(pos_i: &[Point2], pos_j: &[Point2], times: &[f64]) -> Result<()> {
    if pos_i.len() != times.len() || pos_j.len() != times.len() {
        return Err(SpatialError::LengthMismatch(
            "both position series must match times".into(),
        ));
    }
    Ok(())
}

fn accel_times(n_accel: usize, times: &[f64]) -> Vec<f64> {
    // Sample j is stamped at times[j+2] (see derive_scalar_accels).
    (0..n_accel).map(|j| times[j + 2]).collect()
}

fn clock_gaps_ok(times: &[f64], max_gap: f64) -> bool {
    if max_gap <= 0.0 {
        return true;
    }
    times.windows(2).all(|w| {
        let dt = w[1] - w[0];
        dt > 0.0 && dt <= max_gap
    })
}

fn overlap(len_i: usize, len_j: usize, k: isize) -> Option<(usize, usize, usize)> {
    if k >= 0 {
        let k = k as usize;
        if k >= len_j {
            return None;
        }
        let n = (len_i).min(len_j - k);
        if n == 0 { None } else { Some((0, k, n)) }
    } else {
        let k = (-k) as usize;
        if k >= len_i {
            return None;
        }
        let n = (len_j).min(len_i - k);
        if n == 0 { None } else { Some((k, 0, n)) }
    }
}

fn event_counts(ev_i: &[EffortEvent], ev_j: &[EffortEvent], k: isize) -> (usize, usize) {
    let Some((i0, j0, n)) = overlap(ev_i.len(), ev_j.len(), k) else {
        return (0, 0);
    };
    let mut inn = 0usize;
    let mut out = 0usize;
    for t in 0..n {
        match (ev_i[i0 + t], ev_j[j0 + t]) {
            (EffortEvent::Accel, EffortEvent::Accel) | (EffortEvent::Decel, EffortEvent::Decel) => inn += 1,
            (EffortEvent::Accel, EffortEvent::Decel) | (EffortEvent::Decel, EffortEvent::Accel) => out += 1,
            _ => {}
        }
    }
    (inn, out)
}

fn sign_agree_fraction(a_i: &[f64], a_j: &[f64], k: isize) -> Option<f64> {
    let (i0, j0, n) = overlap(a_i.len(), a_j.len(), k)?;
    let mut agree = 0usize;
    let mut used = 0usize;
    for t in 0..n {
        let ai = a_i[i0 + t];
        let aj = a_j[j0 + t];
        if ai.abs() <= ACCEL_EPS || aj.abs() <= ACCEL_EPS {
            continue;
        }
        used += 1;
        if ai.signum() == aj.signum() {
            agree += 1;
        }
    }
    if used == 0 {
        None
    } else {
        Some(agree as f64 / used as f64)
    }
}

fn event_fraction(inn: usize, out: usize) -> Option<f64> {
    let den = inn + out;
    if den == 0 { None } else { Some(inn as f64 / den as f64) }
}

fn lag_samples(accel_t: &[f64], lag_sec: f64) -> Vec<isize> {
    if lag_sec <= 0.0 || accel_t.len() < 2 {
        return vec![0];
    }
    let t0 = accel_t[0];
    let mut ks = vec![0isize];
    for (j, &t) in accel_t.iter().enumerate().skip(1) {
        if (t - t0).abs() <= lag_sec + 1e-12 {
            ks.push(j as isize);
            ks.push(-(j as isize));
        } else {
            break;
        }
    }
    ks.sort_unstable();
    ks.dedup();
    ks
}

fn score_lag(
    ev_i: &[EffortEvent],
    ev_j: &[EffortEvent],
    a_i: &[f64],
    a_j: &[f64],
    k: isize,
) -> (f64, f64, usize, usize, Option<f64>) {
    let (inn, out) = event_counts(ev_i, ev_j, k);
    let ev_frac = event_fraction(inn, out).unwrap_or(-1.0);
    let sign = sign_agree_fraction(a_i, a_j, k);
    let sign_key = sign.unwrap_or(-1.0);
    (ev_frac, sign_key, inn, out, sign)
}

fn best_alignment(
    ev_i: &[EffortEvent],
    ev_j: &[EffortEvent],
    a_i: &[f64],
    a_j: &[f64],
    accel_t: &[f64],
    lag_sec: f64,
) -> Option<(isize, usize, usize, Option<f64>, f64)> {
    let mut best: Option<(f64, f64, isize, usize, usize, Option<f64>)> = None;
    for k in lag_samples(accel_t, lag_sec) {
        let (ev_frac, sign_key, inn, out, sign) = score_lag(ev_i, ev_j, a_i, a_j, k);
        if inn + out == 0 && sign.is_none() {
            continue;
        }
        let cand = (ev_frac, sign_key, k, inn, out, sign);
        match best {
            None => best = Some(cand),
            Some(cur) => {
                if cand.0 > cur.0 || (cand.0 == cur.0 && cand.1 > cur.1) {
                    best = Some(cand);
                }
            }
        }
    }
    best.map(|(_, _, k, inn, out, sign)| {
        let lag = if k == 0 || accel_t.len() < 2 {
            0.0
        } else {
            let idx = k.unsigned_abs();
            if idx < accel_t.len() {
                (accel_t[idx] - accel_t[0]) * if k >= 0 { 1.0 } else { -1.0 }
            } else {
                0.0
            }
        };
        (k, inn, out, sign, lag)
    })
}

fn headings_for_accels(pos: &[Point2], times: &[f64], min_speed: f64) -> Vec<Option<f64>> {
    let vels = derive_velocities_from_times(pos, times);
    let heads = derive_headings(&vels, min_speed);
    if heads.len() < 2 {
        return Vec::new();
    }
    // Accel sample j uses the later velocity interval (index j+1).
    heads.into_iter().skip(1).collect()
}

/// Effort-phase from two aligned position series sharing `times`.
pub fn pairwise_effort_phase(
    pos_i: &[Point2],
    pos_j: &[Point2],
    times: &[f64],
    cfg: &PhaseWindow,
) -> Result<Option<PairwiseEffortPhase>> {
    validate_cfg(cfg)?;
    require_aligned(pos_i, pos_j, times)?;
    let speeds_i = derive_speeds(pos_i, times);
    let speeds_j = derive_speeds(pos_j, times);
    let a_i = derive_scalar_accels(&speeds_i, times);
    let a_j = derive_scalar_accels(&speeds_j, times);
    if a_i.len() != a_j.len() {
        return Err(SpatialError::LengthMismatch(
            "acceleration series lengths differ".into(),
        ));
    }
    let ev_i = accel_decel_events(&speeds_i, times, cfg.accel_threshold);
    let ev_j = accel_decel_events(&speeds_j, times, cfg.accel_threshold);
    let accel_t = accel_times(a_i.len(), times);
    if !clock_gaps_ok(times, cfg.max_gap_sec) {
        return Ok(None);
    }
    let Some((_k, inn, out, sign, lag)) = best_alignment(&ev_i, &ev_j, &a_i, &a_j, &accel_t, cfg.lag_sec) else {
        return Ok(None);
    };
    Ok(Some(PairwiseEffortPhase {
        in_phase_events: inn,
        out_of_phase_events: out,
        event_in_phase_fraction: event_fraction(inn, out),
        sign_agree_fraction: sign,
        lag_sec: lag,
    }))
}

/// Directional-phase from two aligned position series sharing `times`.
///
/// Uses the same lag search as [`pairwise_effort_phase`]. Samples without a
/// heading (near-zero speed) increment `heading_undefined` and are not scored.
pub fn pairwise_directional_phase(
    pos_i: &[Point2],
    pos_j: &[Point2],
    times: &[f64],
    cfg: &PhaseWindow,
) -> Result<Option<PairwiseDirectionalPhase>> {
    validate_cfg(cfg)?;
    require_aligned(pos_i, pos_j, times)?;
    let speeds_i = derive_speeds(pos_i, times);
    let speeds_j = derive_speeds(pos_j, times);
    let a_i = derive_scalar_accels(&speeds_i, times);
    let a_j = derive_scalar_accels(&speeds_j, times);
    let ev_i = accel_decel_events(&speeds_i, times, cfg.accel_threshold);
    let ev_j = accel_decel_events(&speeds_j, times, cfg.accel_threshold);
    let accel_t = accel_times(a_i.len(), times);
    if !clock_gaps_ok(times, cfg.max_gap_sec) {
        return Ok(None);
    }
    let Some((k, _, _, _, _)) = best_alignment(&ev_i, &ev_j, &a_i, &a_j, &accel_t, cfg.lag_sec) else {
        return Ok(None);
    };
    let h_i = headings_for_accels(pos_i, times, cfg.min_speed);
    let h_j = headings_for_accels(pos_j, times, cfg.min_speed);
    if h_i.len() != a_i.len() || h_j.len() != a_j.len() {
        return Ok(None);
    }
    let Some((i0, j0, n)) = overlap(a_i.len(), a_j.len(), k) else {
        return Ok(None);
    };

    let mut counts = [0usize; 4];
    let mut heading_undefined = 0usize;
    for t in 0..n {
        let ai = a_i[i0 + t];
        let aj = a_j[j0 + t];
        if ai.abs() <= ACCEL_EPS || aj.abs() <= ACCEL_EPS {
            continue;
        }
        let same_effort = ai.signum() == aj.signum();
        match (h_i[i0 + t], h_j[j0 + t]) {
            (Some(hi), Some(hj)) => {
                let same_heading = angular_diff(hi, hj).abs() <= cfg.heading_gate_rad;
                let rel = DirectionalRelation::from_bools(same_effort, same_heading);
                counts[rel.index()] += 1;
            }
            _ => heading_undefined += 1,
        }
    }
    if counts.iter().all(|&c| c == 0) && heading_undefined == 0 {
        return Ok(None);
    }
    Ok(Some(PairwiseDirectionalPhase::from_counts(counts, heading_undefined)))
}

fn window_range(accel_t: &[f64], center: usize, window_sec: f64) -> (usize, usize) {
    if accel_t.is_empty() || center >= accel_t.len() {
        return (0, 0);
    }
    if window_sec <= 0.0 {
        return (0, accel_t.len());
    }
    let tc = accel_t[center];
    let lo = accel_t.iter().position(|&t| t + 1e-12 >= tc - window_sec).unwrap_or(0);
    let hi = accel_t
        .iter()
        .rposition(|&t| t <= tc + window_sec + 1e-12)
        .map(|i| i + 1)
        .unwrap_or(0);
    if hi > lo { (lo, hi) } else { (0, 0) }
}

fn score_effort_range(p: &PairPrep, lo: usize, hi: usize, lag_sec: f64) -> Option<PairwiseEffortPhase> {
    if hi <= lo || hi > p.ev_i.len() {
        return None;
    }
    let (_k, inn, out, sign, lag) = best_alignment(
        &p.ev_i[lo..hi],
        &p.ev_j[lo..hi],
        &p.a_i[lo..hi],
        &p.a_j[lo..hi],
        &p.accel_t[lo..hi],
        lag_sec,
    )?;
    Some(PairwiseEffortPhase {
        in_phase_events: inn,
        out_of_phase_events: out,
        event_in_phase_fraction: event_fraction(inn, out),
        sign_agree_fraction: sign,
        lag_sec: lag,
    })
}

fn score_directional_range(p: &PairPrep, lo: usize, hi: usize, cfg: &PhaseWindow) -> Option<PairwiseDirectionalPhase> {
    if hi <= lo || p.h_i.len() != p.a_i.len() || p.h_j.len() != p.a_j.len() {
        return None;
    }
    let (k, _, _, _, _) = best_alignment(
        &p.ev_i[lo..hi],
        &p.ev_j[lo..hi],
        &p.a_i[lo..hi],
        &p.a_j[lo..hi],
        &p.accel_t[lo..hi],
        cfg.lag_sec,
    )?;
    let (i0, j0, n) = overlap(hi - lo, hi - lo, k)?;
    let mut counts = [0usize; 4];
    let mut heading_undefined = 0usize;
    for t in 0..n {
        let ii = lo + i0 + t;
        let jj = lo + j0 + t;
        let ai = p.a_i[ii];
        let aj = p.a_j[jj];
        if ai.abs() <= ACCEL_EPS || aj.abs() <= ACCEL_EPS {
            continue;
        }
        let same_effort = ai.signum() == aj.signum();
        match (p.h_i[ii], p.h_j[jj]) {
            (Some(hi_h), Some(hj_h)) => {
                let same_heading = angular_diff(hi_h, hj_h).abs() <= cfg.heading_gate_rad;
                counts[DirectionalRelation::from_bools(same_effort, same_heading).index()] += 1;
            }
            _ => heading_undefined += 1,
        }
    }
    if counts.iter().all(|&c| c == 0) && heading_undefined == 0 {
        return None;
    }
    Some(PairwiseDirectionalPhase::from_counts(counts, heading_undefined))
}

fn score_closing_range(c_ij: &[Option<f64>], c_ji: &[Option<f64>], lo: usize, hi: usize) -> Option<PairwiseClosing> {
    if hi <= lo || c_ij.len() != c_ji.len() || hi > c_ij.len() {
        return None;
    }
    let mut sum_ij = 0.0;
    let mut n_ij = 0usize;
    let mut sum_ji = 0.0;
    let mut n_ji = 0usize;
    let mut both_c = 0usize;
    let mut both_o = 0usize;
    let mut mixed = 0usize;
    let mut paired = 0usize;
    for t in lo..hi {
        if let Some(a) = c_ij[t] {
            sum_ij += a;
            n_ij += 1;
        }
        if let Some(b) = c_ji[t] {
            sum_ji += b;
            n_ji += 1;
        }
        match (c_ij[t], c_ji[t]) {
            (Some(a), Some(b)) if a.abs() > ACCEL_EPS && b.abs() > ACCEL_EPS => {
                paired += 1;
                let ac = a > 0.0;
                let bc = b > 0.0;
                if ac && bc {
                    both_c += 1;
                } else if !ac && !bc {
                    both_o += 1;
                } else {
                    mixed += 1;
                }
            }
            _ => {}
        }
    }
    if n_ij == 0 && n_ji == 0 && paired == 0 {
        return None;
    }
    let frac = |k: usize| {
        if paired == 0 {
            None
        } else {
            Some(k as f64 / paired as f64)
        }
    };
    Some(PairwiseClosing {
        mean_i_toward_j: if n_ij > 0 { Some(sum_ij / n_ij as f64) } else { None },
        mean_j_toward_i: if n_ji > 0 { Some(sum_ji / n_ji as f64) } else { None },
        both_closing_frac: frac(both_c),
        both_opening_frac: frac(both_o),
        mixed_frac: frac(mixed),
    })
}

struct PairPrep {
    a_i: Vec<f64>,
    a_j: Vec<f64>,
    ev_i: Vec<EffortEvent>,
    ev_j: Vec<EffortEvent>,
    accel_t: Vec<f64>,
    h_i: Vec<Option<f64>>,
    h_j: Vec<Option<f64>>,
    c_ij: Vec<Option<f64>>,
    c_ji: Vec<Option<f64>>,
}

fn prep_pair(pos_i: &[Point2], pos_j: &[Point2], times: &[f64], cfg: &PhaseWindow) -> Result<Option<PairPrep>> {
    validate_cfg(cfg)?;
    require_aligned(pos_i, pos_j, times)?;
    if !clock_gaps_ok(times, cfg.max_gap_sec) {
        return Ok(None);
    }
    let speeds_i = derive_speeds(pos_i, times);
    let speeds_j = derive_speeds(pos_j, times);
    let a_i = derive_scalar_accels(&speeds_i, times);
    let a_j = derive_scalar_accels(&speeds_j, times);
    if a_i.len() != a_j.len() {
        return Err(SpatialError::LengthMismatch(
            "acceleration series lengths differ".into(),
        ));
    }
    let ev_i = accel_decel_events(&speeds_i, times, cfg.accel_threshold);
    let ev_j = accel_decel_events(&speeds_j, times, cfg.accel_threshold);
    let accel_t = accel_times(a_i.len(), times);
    let h_i = headings_for_accels(pos_i, times, cfg.min_speed);
    let h_j = headings_for_accels(pos_j, times, cfg.min_speed);
    let c_ij = crate::kinematics::derive_closing_accels(pos_i, pos_j, times, cfg.min_speed);
    let c_ji = crate::kinematics::derive_closing_accels(pos_j, pos_i, times, cfg.min_speed);
    Ok(Some(PairPrep {
        a_i,
        a_j,
        ev_i,
        ev_j,
        accel_t,
        h_i,
        h_j,
        c_ij,
        c_ji,
    }))
}

/// Map a position-frame index to an accel-sample index (`times[j+2]`).
pub fn accel_index_for_frame(frame_idx: usize) -> Option<usize> {
    frame_idx.checked_sub(2)
}

/// Effort-phase of one pair at every accel sample (`[t−W, t+W]` when `window_sec > 0`).
///
/// Length is `positions.len().saturating_sub(2)`, not `times.len()`.
pub fn pairwise_effort_phase_series(
    pos_i: &[Point2],
    pos_j: &[Point2],
    times: &[f64],
    cfg: &PhaseWindow,
) -> Result<Vec<Option<PairwiseEffortPhase>>> {
    let Some(p) = prep_pair(pos_i, pos_j, times, cfg)? else {
        return Ok(Vec::new());
    };
    let n = p.accel_t.len();
    let mut out = Vec::with_capacity(n);
    for c in 0..n {
        let (lo, hi) = window_range(&p.accel_t, c, cfg.window_sec);
        out.push(score_effort_range(&p, lo, hi, cfg.lag_sec));
    }
    Ok(out)
}

/// Directional-phase series; same length and windowing as [`pairwise_effort_phase_series`].
pub fn pairwise_directional_phase_series(
    pos_i: &[Point2],
    pos_j: &[Point2],
    times: &[f64],
    cfg: &PhaseWindow,
) -> Result<Vec<Option<PairwiseDirectionalPhase>>> {
    let Some(p) = prep_pair(pos_i, pos_j, times, cfg)? else {
        return Ok(Vec::new());
    };
    let n = p.accel_t.len();
    let mut out = Vec::with_capacity(n);
    for c in 0..n {
        let (lo, hi) = window_range(&p.accel_t, c, cfg.window_sec);
        out.push(score_directional_range(&p, lo, hi, cfg));
    }
    Ok(out)
}

/// Closing series (`i→j` and `j→i` means per window). Same length as effort series.
pub fn pairwise_closing_series(
    pos_i: &[Point2],
    pos_j: &[Point2],
    times: &[f64],
    cfg: &PhaseWindow,
) -> Result<Vec<Option<PairwiseClosing>>> {
    let Some(p) = prep_pair(pos_i, pos_j, times, cfg)? else {
        return Ok(Vec::new());
    };
    let n = p.accel_t.len();
    let mut out = Vec::with_capacity(n);
    for c in 0..n {
        let (lo, hi) = window_range(&p.accel_t, c, cfg.window_sec);
        out.push(score_closing_range(&p.c_ij, &p.c_ji, lo, hi));
    }
    Ok(out)
}

/// Session closing score for one pair (full series).
pub fn pairwise_closing(
    pos_i: &[Point2],
    pos_j: &[Point2],
    times: &[f64],
    cfg: &PhaseWindow,
) -> Result<Option<PairwiseClosing>> {
    let Some(p) = prep_pair(pos_i, pos_j, times, cfg)? else {
        return Ok(None);
    };
    Ok(score_closing_range(&p.c_ij, &p.c_ji, 0, p.c_ij.len()))
}

fn empty_pair_matrix<T: Clone>(n: usize) -> Vec<Vec<Option<T>>> {
    vec![vec![None; n]; n]
}

fn frame_center(frame_idx: usize, n_accel: usize) -> Option<usize> {
    let c = accel_index_for_frame(frame_idx)?;
    if c < n_accel { Some(c) } else { None }
}

/// All-pairs effort-phase at one position-frame index (`n × n`, diagonal `None`).
pub fn pairwise_effort_phase_at(
    positions: &[Vec<Point2>],
    times: &[f64],
    frame_idx: usize,
    cfg: &PhaseWindow,
) -> Result<Vec<Vec<Option<PairwiseEffortPhase>>>> {
    let n = positions.len();
    let mut mat = empty_pair_matrix(n);
    if n == 0 {
        return Ok(mat);
    }
    let n_accel = times.len().saturating_sub(2);
    let Some(center) = frame_center(frame_idx, n_accel) else {
        return Ok(mat);
    };
    for i in 0..n {
        for j in (i + 1)..n {
            let Some(p) = prep_pair(&positions[i], &positions[j], times, cfg)? else {
                continue;
            };
            let (lo, hi) = window_range(&p.accel_t, center, cfg.window_sec);
            let score = score_effort_range(&p, lo, hi, cfg.lag_sec);
            mat[i][j] = score.clone();
            mat[j][i] = score;
        }
    }
    Ok(mat)
}

/// All-pairs directional-phase at one position-frame index.
pub fn pairwise_directional_phase_at(
    positions: &[Vec<Point2>],
    times: &[f64],
    frame_idx: usize,
    cfg: &PhaseWindow,
) -> Result<Vec<Vec<Option<PairwiseDirectionalPhase>>>> {
    let n = positions.len();
    let mut mat = empty_pair_matrix(n);
    if n == 0 {
        return Ok(mat);
    }
    let n_accel = times.len().saturating_sub(2);
    let Some(center) = frame_center(frame_idx, n_accel) else {
        return Ok(mat);
    };
    for i in 0..n {
        for j in (i + 1)..n {
            let Some(p) = prep_pair(&positions[i], &positions[j], times, cfg)? else {
                continue;
            };
            let (lo, hi) = window_range(&p.accel_t, center, cfg.window_sec);
            let score = score_directional_range(&p, lo, hi, cfg);
            mat[i][j] = score.clone();
            mat[j][i] = score;
        }
    }
    Ok(mat)
}

/// All-pairs closing at one position-frame index.
pub fn pairwise_closing_at(
    positions: &[Vec<Point2>],
    times: &[f64],
    frame_idx: usize,
    cfg: &PhaseWindow,
) -> Result<Vec<Vec<Option<PairwiseClosing>>>> {
    let n = positions.len();
    let mut mat = empty_pair_matrix(n);
    if n == 0 {
        return Ok(mat);
    }
    let n_accel = times.len().saturating_sub(2);
    let Some(center) = frame_center(frame_idx, n_accel) else {
        return Ok(mat);
    };
    for i in 0..n {
        for j in (i + 1)..n {
            let Some(p) = prep_pair(&positions[i], &positions[j], times, cfg)? else {
                continue;
            };
            let (lo, hi) = window_range(&p.accel_t, center, cfg.window_sec);
            let score = score_closing_range(&p.c_ij, &p.c_ji, lo, hi);
            mat[i][j] = score.clone();
            // Reverse pair: swap means so mat[j][i] is j-toward-i as "i_toward_j" of that view.
            mat[j][i] = score.map(|s| PairwiseClosing {
                mean_i_toward_j: s.mean_j_toward_i,
                mean_j_toward_i: s.mean_i_toward_j,
                both_closing_frac: s.both_closing_frac,
                both_opening_frac: s.both_opening_frac,
                mixed_frac: s.mixed_frac,
            });
        }
    }
    Ok(mat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{
        Point2,
        Vec2,
    };

    fn cfg_thresh(th: f64) -> PhaseWindow {
        PhaseWindow {
            accel_threshold: th,
            ..PhaseWindow::default()
        }
    }

    /// Positions along `dir` with the given interval speeds (m/s) at dt = 1 s.
    fn from_speeds(start: Point2, dir: Vec2, speeds: &[f64]) -> (Vec<f64>, Vec<Point2>) {
        let unit = dir.normalize();
        let mut times = vec![0.0];
        let mut pos = vec![start];
        let mut p = start;
        for (i, &s) in speeds.iter().enumerate() {
            p = p + unit * s;
            times.push((i + 1) as f64);
            pos.push(p);
        }
        (times, pos)
    }

    #[test]
    fn together_in_phase() {
        let speeds = [1.0, 2.0, 1.0];
        let (times, a) = from_speeds(Point2::origin(), Vec2::new(1.0, 0.0), &speeds);
        let (_, b) = from_speeds(Point2::new(0.0, 2.0), Vec2::new(1.0, 0.0), &speeds);
        let cfg = cfg_thresh(0.4);
        let effort = pairwise_effort_phase(&a, &b, &times, &cfg).unwrap().unwrap();
        assert!(effort.in_phase_events >= 1);
        assert_eq!(effort.out_of_phase_events, 0);
        assert_eq!(effort.event_in_phase_fraction, Some(1.0));
        let dir = pairwise_directional_phase(&a, &b, &times, &cfg).unwrap().unwrap();
        assert_eq!(dir.dominant, Some(DirectionalRelation::InPhase));
    }

    #[test]
    fn spatially_opposed_same_effort() {
        let speeds = [1.0, 2.5, 2.5];
        let (times, a) = from_speeds(Point2::origin(), Vec2::new(1.0, 0.0), &speeds);
        let (_, b) = from_speeds(Point2::new(0.0, 2.0), Vec2::new(-1.0, 0.0), &speeds);
        let cfg = cfg_thresh(0.4);
        let effort = pairwise_effort_phase(&a, &b, &times, &cfg).unwrap().unwrap();
        assert!(effort.in_phase_events >= 1);
        assert_eq!(effort.out_of_phase_events, 0);
        let dir = pairwise_directional_phase(&a, &b, &times, &cfg).unwrap().unwrap();
        assert_eq!(dir.dominant, Some(DirectionalRelation::SpatiallyOpposed));
    }

    #[test]
    fn one_go_one_brake() {
        let (times, a) = from_speeds(Point2::origin(), Vec2::new(1.0, 0.0), &[1.0, 2.5]);
        let (_, b) = from_speeds(Point2::new(0.0, 2.0), Vec2::new(1.0, 0.0), &[2.5, 1.0]);
        let cfg = cfg_thresh(0.4);
        let effort = pairwise_effort_phase(&a, &b, &times, &cfg).unwrap().unwrap();
        assert_eq!(effort.in_phase_events, 0);
        assert!(effort.out_of_phase_events >= 1);
        let dir = pairwise_directional_phase(&a, &b, &times, &cfg).unwrap().unwrap();
        assert_eq!(dir.dominant, Some(DirectionalRelation::OutOfPhase));
    }

    #[test]
    fn high_min_speed_drops_directional_heading() {
        let speeds = [1.0, 2.0, 1.0];
        let (times, a) = from_speeds(Point2::origin(), Vec2::new(1.0, 0.0), &speeds);
        let (_, b) = from_speeds(Point2::new(0.0, 2.0), Vec2::new(1.0, 0.0), &speeds);
        let mut cfg = cfg_thresh(0.4);
        cfg.min_speed = 100.0;
        let effort = pairwise_effort_phase(&a, &b, &times, &cfg).unwrap().unwrap();
        assert!(effort.in_phase_events >= 1);
        let dir = pairwise_directional_phase(&a, &b, &times, &cfg).unwrap().unwrap();
        assert!(dir.heading_undefined > 0);
        assert!(dir.dominant.is_none());
    }

    #[test]
    fn clock_gap_returns_none() {
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(4.0, 0.0),
        ];
        let b = vec![
            Point2::new(0.0, 1.0),
            Point2::new(1.0, 1.0),
            Point2::new(3.0, 1.0),
            Point2::new(4.0, 1.0),
        ];
        let times = vec![0.0, 1.0, 10.0, 11.0];
        let mut cfg = cfg_thresh(0.4);
        cfg.max_gap_sec = 2.0;
        assert!(pairwise_effort_phase(&a, &b, &times, &cfg).unwrap().is_none());
    }

    #[test]
    fn length_mismatch_errors() {
        let a = vec![Point2::origin(), Point2::new(1.0, 0.0)];
        let b = vec![Point2::origin()];
        let times = vec![0.0, 1.0];
        let err = pairwise_effort_phase(&a, &b, &times, &PhaseWindow::default()).unwrap_err();
        assert!(matches!(err, SpatialError::LengthMismatch(_)));
    }

    #[test]
    fn together_then_split_windows() {
        // First half both speed up; then A keeps going, B brakes.
        let (times, a) = from_speeds(Point2::origin(), Vec2::new(1.0, 0.0), &[1.0, 2.0, 2.0, 3.0, 3.5]);
        let (_, b) = from_speeds(Point2::new(0.0, 2.0), Vec2::new(1.0, 0.0), &[1.0, 2.0, 2.0, 1.0, 0.4]);
        let mut cfg = cfg_thresh(0.4);
        cfg.window_sec = 1.0;
        let series = pairwise_effort_phase_series(&a, &b, &times, &cfg).unwrap();
        assert!(!series.is_empty());
        let early = series.first().and_then(|s| s.as_ref()).unwrap();
        let late = series.last().and_then(|s| s.as_ref()).unwrap();
        assert!(
            early.event_in_phase_fraction.unwrap_or(0.0) > late.event_in_phase_fraction.unwrap_or(1.0)
                || late.out_of_phase_events > early.out_of_phase_events
        );
        let session = pairwise_effort_phase(&a, &b, &times, &cfg).unwrap().unwrap();
        assert!(session.in_phase_events > 0 && session.out_of_phase_events > 0);

        let at_early = pairwise_effort_phase_at(&[a.clone(), b.clone()], &times, 2, &cfg).unwrap();
        assert!(at_early[0][1].is_some());
        let at_pre = pairwise_effort_phase_at(&[a, b], &times, 0, &cfg).unwrap();
        assert!(at_pre[0][1].is_none());
        assert!(accel_index_for_frame(0).is_none());
        assert_eq!(accel_index_for_frame(2), Some(0));
    }

    #[test]
    fn closing_toward_stationary_partner() {
        // A speeds up toward B sitting at x=10.
        let times = vec![0.0, 1.0, 2.0, 3.0];
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(6.0, 0.0),
        ];
        let b = vec![Point2::new(10.0, 0.0); 4];
        let cfg = cfg_thresh(0.4);
        let close = crate::kinematics::derive_closing_accels(&a, &b, &times, 0.1);
        assert!(close.iter().flatten().any(|&c| c > 0.0));
        let back = crate::kinematics::derive_closing_accels(&b, &a, &times, 0.1);
        assert!(back.iter().all(|c| c.is_none() || c.unwrap().abs() < 1e-9));
        let pair = pairwise_closing(&a, &b, &times, &cfg).unwrap().unwrap();
        assert!(pair.mean_i_toward_j.unwrap() > 0.0);
    }
}
