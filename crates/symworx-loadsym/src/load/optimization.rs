// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Multi-day load planning on top of the pulse-response model.
//!
//! Given recent daily loads (e.g. TSS), choose a short horizon plan
//! (default 3 days) aimed at one of:
//! - [`LoadGoal::Recovery`] — active recovery: mean load ~25–55% of chronic
//! - [`LoadGoal::Maintenance`] — mean load ~85–115% of chronic, even days
//! - [`LoadGoal::Overload`] — mean load ~115–140% of chronic, with variety
//!
//! **Primary success is chronic-relative mean load** (scale-free). Form (TSB)
//! and ACWR are soft / separate context — they do not alone hard-fail a plan.
//!
//! Scoring prefers **realistic structure** (not pure rest for recovery, not
//! all-max for overload): target load fractions + day-to-day variety + limits
//! on consecutive hard days.

use super::{
    acwr::{
        classify_acwr,
        compute_acute_chronic,
    },
    pulse_response::{
        PulseResponseParams,
        PulseResponseSeries,
        PulseResponseState,
        forecast_pulse_response,
        simulate_pulse_response,
        step_pulse_response,
    },
};
use crate::error::{
    LoadSymError,
    Result,
};

/// Training goal for the next few days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoadGoal {
    /// Elevated load vs chronic (progressive stress).
    Overload,
    /// Hold load near chronic mean.
    Maintenance,
    /// Reduced load vs chronic (active recovery / deload).
    Recovery,
}

impl LoadGoal {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoadGoal::Overload => "overload",
            LoadGoal::Maintenance => "maintenance",
            LoadGoal::Recovery => "recovery",
        }
    }
}

/// Thresholds for [`optimize_load_plan`].
///
/// Primary bands are **fractions of chronic mean load** \(C\). Scoring pulls
/// toward mid-band targets and realistic day patterns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptimizationThresholds {
    /// Planning horizon in days (default 4; allowed 1..=[`MAX_HORIZON_DAYS`]).
    pub horizon_days: usize,
    /// Minimum history days for chronic estimate (default 7).
    pub min_history_days: usize,
    /// Cap template daily load as multiple of recent max (default 1.5).
    pub max_load_factor: f64,

    /// Recovery success: mean plan load ≥ this × chronic (default 0.20).
    /// Prevents pure-rest (0,0,0) from counting as the only optimum.
    pub recovery_load_frac_min: f64,
    /// Recovery success: mean plan load ≤ this × chronic (default 0.55).
    pub recovery_load_frac_max: f64,
    /// Recovery score center (default 0.38 — active recovery).
    pub recovery_load_frac_target: f64,
    /// Soft form note vs rest trajectory (default 0.35).
    pub recovery_form_gain_soft_frac: f64,

    /// Maintenance: mean plan load ≥ this × chronic (default 0.85).
    pub maintenance_load_frac_lo: f64,
    /// Maintenance: mean plan load ≤ this × chronic (default 1.15).
    pub maintenance_load_frac_hi: f64,
    /// Soft weight pulling mean load toward 1.0×C (default 0.5).
    pub maintenance_mean_weight: f64,
    /// Target coefficient of variation (std/mean) for maintenance days (default 0.28).
    pub maintenance_cv_target: f64,
    /// Soft weight on |cv − target| (default 0.55).
    pub maintenance_cv_weight: f64,
    /// Extra penalty when all days are nearly equal (default 0.45).
    pub maintenance_flat_penalty: f64,
    /// Soft: penalize days below this × C (default 0.30) — avoid pure rest in maintain.
    pub maintenance_day_frac_min: f64,
    /// Soft: penalize days above this × C (default 1.35) — avoid race-day spikes.
    pub maintenance_day_frac_max: f64,

    /// Overload: mean plan load ≥ this × chronic (default 1.15).
    pub overload_load_frac_lo: f64,
    /// Overload: mean plan load ≤ this × chronic (default 1.40).
    pub overload_load_frac_hi: f64,
    /// Overload score center as fraction of chronic (default 1.25).
    pub overload_target_load_frac: f64,
    /// Soft weight on consecutive hard days (default 0.20).
    pub overload_consecutive_hard_weight: f64,
    /// Day is "hard" if load ≥ this × chronic (default 1.20).
    pub hard_day_frac: f64,
    /// Soft max consecutive hard days before penalty grows (default 2).
    pub max_consecutive_hard_soft: usize,

    /// When true, attach projected ACWR as **context** (never hard-fails success).
    pub report_acwr: bool,
}

/// Maximum supported plan horizon (days). Longer horizons use beam search
/// rather than full template enumeration (see [`optimize_load_plan`]).
pub const MAX_HORIZON_DAYS: usize = 10;

/// Full enumeration only when `n_templates^H` is at most this (keeps UI snappy).
const FULL_ENUM_MAX: u64 = 20_000;

/// Beam width when H is large (7 templates × 10 days is ~2.8e8 — not enumerable).
const BEAM_WIDTH: usize = 64;

impl Default for OptimizationThresholds {
    fn default() -> Self {
        Self {
            horizon_days: 4,
            min_history_days: 7,
            max_load_factor: 1.5,
            recovery_load_frac_min: 0.20,
            recovery_load_frac_max: 0.55,
            recovery_load_frac_target: 0.38,
            recovery_form_gain_soft_frac: 0.35,
            maintenance_load_frac_lo: 0.85,
            maintenance_load_frac_hi: 1.15,
            maintenance_mean_weight: 0.5,
            maintenance_cv_target: 0.28,
            maintenance_cv_weight: 0.55,
            maintenance_flat_penalty: 0.45,
            maintenance_day_frac_min: 0.30,
            maintenance_day_frac_max: 1.35,
            overload_load_frac_lo: 1.15,
            overload_load_frac_hi: 1.40,
            overload_target_load_frac: 1.25,
            overload_consecutive_hard_weight: 0.20,
            hard_day_frac: 1.20,
            max_consecutive_hard_soft: 2,
            report_acwr: true,
        }
    }
}

/// Result of multi-day load planning.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadPlan {
    /// Recommended daily TSS (or load units) for each horizon day.
    pub daily_tss: Vec<f64>,
    /// Predicted pulse-response states over the plan horizon only.
    pub predicted_states: PulseResponseSeries,
    pub goal: LoadGoal,
    /// Whether the plan meets **load-band** success criteria for the goal.
    pub success: bool,
    /// Lower is better (used for ranking candidate sequences).
    pub score: f64,
    /// Human-readable notes (goal assessment, form soft notes, ACWR context).
    pub messages: Vec<String>,
    /// Form at start of plan (after history).
    pub form_start: f64,
    /// Form at end of plan.
    pub form_end: f64,
    /// Chronic load estimate used to scale templates and bands.
    pub chronic_ref: f64,
    /// Mean planned daily load.
    pub mean_plan_load: f64,
    /// \(\bar w / C\) when \(C > 0\).
    pub load_frac: f64,
    /// Projected ACWR after appending the plan (separate context; optional).
    pub projected_acwr: Option<f64>,
}

/// Day templates as fractions of chronic (finer grid → more realistic mixes).
/// rest · easy · moderate · steady · tempo · hard · long
const TEMPLATE_FRACS: [f64; 7] = [0.0, 0.25, 0.45, 0.70, 1.0, 1.20, 1.40];

/// Optimize a short multi-day load plan for the given goal.
///
/// `history_loads` should be oldest → newest daily loads (TSS).
///
/// **Search strategy:** full enumeration when `templates^H` is small
/// (`≤ 20_000`); otherwise **beam search** (width 64). There is no
/// mathematical “convergence” issue for H ≤ 10 — the plant is a stable
/// linear filter — only combinatorial cost of brute force.
pub fn optimize_load_plan(
    history_loads: &[f64],
    params: &PulseResponseParams,
    goal: LoadGoal,
    thresholds: &OptimizationThresholds,
) -> Result<LoadPlan> {
    params.validate()?;
    if history_loads.is_empty() {
        return Err(LoadSymError::InsufficientData(
            "need at least one historical daily load".into(),
        ));
    }
    if thresholds.horizon_days == 0 || thresholds.horizon_days > MAX_HORIZON_DAYS {
        return Err(LoadSymError::InvalidParameter(format!(
            "horizon_days must be in 1..={} (got {})",
            MAX_HORIZON_DAYS, thresholds.horizon_days
        )));
    }
    if history_loads.len() < thresholds.min_history_days {
        return Err(LoadSymError::InsufficientData(format!(
            "need at least {} days of history (got {})",
            thresholds.min_history_days,
            history_loads.len()
        )));
    }

    let hist = simulate_pulse_response(history_loads, params, None)?;
    let start = hist
        .last_state()
        .ok_or_else(|| LoadSymError::InsufficientData("empty simulation".into()))?;

    let chronic_ref = chronic_reference(history_loads);
    let recent_max = history_loads.iter().copied().fold(0.0_f64, f64::max).max(chronic_ref);
    let w_max = (recent_max * thresholds.max_load_factor).max(1.0);

    let templates: Vec<f64> = TEMPLATE_FRACS
        .iter()
        .map(|f| (f * chronic_ref).clamp(0.0, w_max))
        .collect();

    let h = thresholds.horizon_days;
    let n_t = templates.len() as u64;
    // 7^10 fits in u64; saturating_pow for safety
    let total = n_t.saturating_pow(h as u32);

    // Bundle shared search inputs once — keeps helpers under Clippy's arg limit.
    let ctx = PlanSearchCtx {
        history_loads,
        start,
        params,
        goal,
        thresholds,
        chronic_ref,
    };

    let best = if total > 0 && total <= FULL_ENUM_MAX {
        search_full_enum(&ctx, &templates, h)
    } else {
        search_beam(&ctx, &templates, h, BEAM_WIDTH)
    };

    best.ok_or_else(|| LoadSymError::InvalidValue("no valid load plan candidates under constraints".into()))
}

/// Shared inputs for full-enumeration / beam plan search.
struct PlanSearchCtx<'a> {
    history_loads: &'a [f64],
    start: PulseResponseState,
    params: &'a PulseResponseParams,
    goal: LoadGoal,
    thresholds: &'a OptimizationThresholds,
    chronic_ref: f64,
}

fn search_full_enum(ctx: &PlanSearchCtx<'_>, templates: &[f64], h: usize) -> Option<LoadPlan> {
    let n_t = templates.len();
    let total = n_t.pow(h as u32);
    let mut best: Option<LoadPlan> = None;
    for idx in 0..total {
        let mut seq = Vec::with_capacity(h);
        let mut rem = idx;
        for _ in 0..h {
            seq.push(templates[rem % n_t]);
            rem /= n_t;
        }
        consider_candidate(ctx, &seq, &mut best);
    }
    best
}

/// Beam search: expand day-by-day, keep top `width` partial sequences by rank.
fn search_beam(ctx: &PlanSearchCtx<'_>, templates: &[f64], h: usize, width: usize) -> Option<LoadPlan> {
    // Each beam entry is a partial (or full) day sequence.
    let mut beam: Vec<Vec<f64>> = vec![Vec::new()];
    let mut best: Option<LoadPlan> = None;

    for _day in 0..h {
        let mut scored: Vec<(Vec<f64>, LoadPlan)> = Vec::new();
        for partial in &beam {
            for &t in templates {
                let mut seq = partial.clone();
                seq.push(t);
                if let Some(plan) = evaluate_candidate(
                    ctx.history_loads,
                    &seq,
                    ctx.start,
                    ctx.params,
                    ctx.goal,
                    ctx.thresholds,
                    ctx.chronic_ref,
                ) {
                    // Track global best among complete sequences only at the end;
                    // still use intermediate scores for pruning.
                    scored.push((seq, plan));
                }
            }
        }
        if scored.is_empty() {
            return best;
        }
        scored.sort_by_key(|a| rank_key(&a.1));
        scored.truncate(width);
        beam = scored.into_iter().map(|(s, _)| s).collect();
    }

    for seq in &beam {
        consider_candidate(ctx, seq, &mut best);
    }
    best
}

fn consider_candidate(ctx: &PlanSearchCtx<'_>, seq: &[f64], best: &mut Option<LoadPlan>) {
    if let Some(plan) = evaluate_candidate(
        ctx.history_loads,
        seq,
        ctx.start,
        ctx.params,
        ctx.goal,
        ctx.thresholds,
        ctx.chronic_ref,
    ) {
        let replace = match best {
            None => true,
            Some(b) => rank_key(&plan) < rank_key(b),
        };
        if replace {
            *best = Some(plan);
        }
    }
}

fn rank_key(p: &LoadPlan) -> (u8, i64) {
    let fail = if p.success { 0u8 } else { 1u8 };
    let s = (p.score * 1e6) as i64;
    (fail, s)
}

fn chronic_reference(history: &[f64]) -> f64 {
    let n = history.len();
    let window = n.clamp(1, 28);
    let slice = &history[n - window..];
    let mean = slice.iter().sum::<f64>() / slice.len() as f64;
    if mean.is_finite() && mean > 1.0 { mean } else { 50.0 }
}

/// Mean |w_i − target| / target (scale-free day-to-day fit to a daily target).
fn mean_abs_frac_dev(days: &[f64], target: f64) -> f64 {
    let t = target.max(1.0);
    if days.is_empty() {
        return 0.0;
    }
    days.iter().map(|w| (w - t).abs() / t).sum::<f64>() / days.len() as f64
}

/// Longest run of days with load ≥ thresh.
fn max_consecutive_ge(days: &[f64], thresh: f64) -> usize {
    let mut best = 0usize;
    let mut cur = 0usize;
    for &w in days {
        if w + 1e-9 >= thresh {
            cur += 1;
            best = best.max(cur);
        } else {
            cur = 0;
        }
    }
    best
}

/// Coefficient of variation (std / mean); 0 if mean ≈ 0.
fn coeff_of_variation(days: &[f64]) -> f64 {
    if days.len() < 2 {
        return 0.0;
    }
    let n = days.len() as f64;
    let mean = days.iter().sum::<f64>() / n;
    if mean.abs() < 1e-9 {
        return 0.0;
    }
    let var = days.iter().map(|w| (w - mean).powi(2)).sum::<f64>() / n;
    var.sqrt() / mean.abs()
}

/// True if all days are within `eps_frac` of each other relative to mean.
fn is_nearly_flat(days: &[f64], eps_frac: f64) -> bool {
    if days.len() < 2 {
        return true;
    }
    let mean = days.iter().sum::<f64>() / days.len() as f64;
    let scale = mean.abs().max(1.0);
    let lo = days.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = days.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (hi - lo) / scale < eps_frac
}

fn evaluate_candidate(
    history: &[f64],
    plan_loads: &[f64],
    start: PulseResponseState,
    params: &PulseResponseParams,
    goal: LoadGoal,
    thr: &OptimizationThresholds,
    chronic_ref: f64,
) -> Option<LoadPlan> {
    let predicted = forecast_pulse_response(start, plan_loads, params).ok()?;
    let form_start = start.form;
    let form_end = predicted.last_state()?.form;

    let mean_plan = plan_loads.iter().sum::<f64>() / plan_loads.len().max(1) as f64;
    let c = chronic_ref.max(1.0);
    let load_frac = mean_plan / c;

    let mut messages = Vec::new();
    let (success, score) = match goal {
        LoadGoal::Recovery => {
            // Active recovery band: not pure rest, not moderate training.
            let ok = load_frac + 1e-12 >= thr.recovery_load_frac_min && load_frac <= thr.recovery_load_frac_max + 1e-12;

            let target = thr.recovery_load_frac_target;
            // Prefer mean near target + days near target·C (easy rides, not one big + zeros).
            let day_dev = mean_abs_frac_dev(plan_loads, target * c);
            let mut score = 1.2 * (load_frac - target).abs() + 0.8 * day_dev;

            // Extra push away from pure rest
            let zero_frac = plan_loads.iter().filter(|&&w| w < 1.0).count() as f64 / plan_loads.len().max(1) as f64;
            if zero_frac > 0.67 {
                score += 0.4 * zero_frac;
            }

            let rest_traj = forecast_pulse_response(start, &vec![0.0; plan_loads.len()], params).ok()?;
            let form_rest = rest_traj.last_state()?.form;
            let rest_gain = (form_rest - form_start).max(0.0);
            let gain = form_end - form_start;
            let soft_frac = if rest_gain > 1e-6 {
                gain / rest_gain
            } else if gain >= 0.0 {
                1.0
            } else {
                0.0
            };
            // Mild reward for form recovery
            if gain > 0.0 {
                score -= 0.03 * soft_frac.min(1.0);
            }

            if ok {
                messages.push(format!(
                    "OK recovery: mean load {:.0}% of chronic (band {:.0}–{:.0}%, target ~{:.0}%)",
                    load_frac * 100.0,
                    thr.recovery_load_frac_min * 100.0,
                    thr.recovery_load_frac_max * 100.0,
                    target * 100.0
                ));
            } else {
                messages.push(format!(
                    "FAIL recovery: mean load {:.0}% of chronic (need {:.0}–{:.0}%)",
                    load_frac * 100.0,
                    thr.recovery_load_frac_min * 100.0,
                    thr.recovery_load_frac_max * 100.0
                ));
            }
            messages.push(format!(
                "form soft: {:+.1} ({:.0}% of rest gain; soft ≥ {:.0}%)",
                gain,
                soft_frac * 100.0,
                thr.recovery_form_gain_soft_frac * 100.0
            ));
            (ok, score)
        }
        LoadGoal::Maintenance => {
            let ok =
                load_frac >= thr.maintenance_load_frac_lo - 1e-12 && load_frac <= thr.maintenance_load_frac_hi + 1e-12;
            // Keep weekly/horizon mean near chronic…
            let mut score = thr.maintenance_mean_weight * (load_frac - 1.0).abs();
            // …but prefer modulated days (easy / steady / moderate), not flat TSS.
            let cv = coeff_of_variation(plan_loads);
            score += thr.maintenance_cv_weight * (cv - thr.maintenance_cv_target).abs();
            if is_nearly_flat(plan_loads, 0.08) {
                score += thr.maintenance_flat_penalty;
            }
            // Soft bounds per day: avoid pure rest + race-day spikes that only average to C
            let mut extreme_pen = 0.0;
            for &w in plan_loads {
                let f = w / c;
                if f + 1e-12 < thr.maintenance_day_frac_min {
                    extreme_pen += thr.maintenance_day_frac_min - f;
                }
                if f > thr.maintenance_day_frac_max + 1e-12 {
                    extreme_pen += f - thr.maintenance_day_frac_max;
                }
            }
            score += 0.4 * extreme_pen;

            if ok {
                messages.push(format!(
                    "OK maintenance: mean load {:.0}% of chronic (band {:.0}–{:.0}%)",
                    load_frac * 100.0,
                    thr.maintenance_load_frac_lo * 100.0,
                    thr.maintenance_load_frac_hi * 100.0
                ));
            } else {
                messages.push(format!(
                    "FAIL maintenance: mean load {:.0}% of chronic (need {:.0}–{:.0}%)",
                    load_frac * 100.0,
                    thr.maintenance_load_frac_lo * 100.0,
                    thr.maintenance_load_frac_hi * 100.0
                ));
            }
            messages.push(format!(
                "structure soft: CV={:.2} (target ~{:.2}; modulate days, avoid flat)",
                cv, thr.maintenance_cv_target
            ));
            (ok, score)
        }
        LoadGoal::Overload => {
            let ok = load_frac >= thr.overload_load_frac_lo - 1e-12
                && load_frac <= thr.overload_load_frac_hi + 1e-12
                && mean_plan > c;
            let target = thr.overload_target_load_frac;
            let mut score = 1.1 * (load_frac - target).abs();
            // Prefer days near target·C rather than all-long or one spike
            let day_dev = mean_abs_frac_dev(plan_loads, target * c);
            score += 0.45 * day_dev;

            let hard_thr = thr.hard_day_frac * c;
            let cons = max_consecutive_ge(plan_loads, hard_thr);
            if cons > thr.max_consecutive_hard_soft {
                score += thr.overload_consecutive_hard_weight * (cons - thr.max_consecutive_hard_soft) as f64;
                messages.push(format!(
                    "structure soft: {} consecutive hard days (≥{:.0}%C); prefer ≤{}",
                    cons,
                    thr.hard_day_frac * 100.0,
                    thr.max_consecutive_hard_soft
                ));
            } else {
                messages.push(format!(
                    "structure soft: max consecutive hard days = {} (ok ≤{})",
                    cons, thr.max_consecutive_hard_soft
                ));
            }

            if form_end > form_start + 1.0 {
                score += 0.1;
                messages.push(format!(
                    "form soft: form rising {:+.1} under elevated load",
                    form_end - form_start
                ));
            } else {
                messages.push(format!(
                    "form soft: form {:+.1} (no absolute TSB dip required)",
                    form_end - form_start
                ));
            }

            if ok {
                messages.push(format!(
                    "OK overload: mean load {:.0}% of chronic (band {:.0}–{:.0}%, target ~{:.0}%)",
                    load_frac * 100.0,
                    thr.overload_load_frac_lo * 100.0,
                    thr.overload_load_frac_hi * 100.0,
                    target * 100.0
                ));
            } else {
                messages.push(format!(
                    "FAIL overload: mean load {:.0}% of chronic (need {:.0}–{:.0}% and > chronic)",
                    load_frac * 100.0,
                    thr.overload_load_frac_lo * 100.0,
                    thr.overload_load_frac_hi * 100.0
                ));
            }
            (ok, score)
        }
    };

    // ACWR: separate context only
    let mut projected_acwr = None;
    if thr.report_acwr {
        let mut extended: Vec<f64> = history.to_vec();
        extended.extend_from_slice(plan_loads);
        match compute_acute_chronic(&extended, 7, 28) {
            Ok(s) => {
                projected_acwr = Some(s.acwr);
                messages.push(format!(
                    "ACWR context: projected={:.2} ({}) — advisory only",
                    s.acwr,
                    classify_acwr(s.acwr).as_str()
                ));
            }
            Err(_) => {
                messages.push("ACWR context: insufficient history for 7/28 window (advisory skipped)".into());
            }
        }
    }

    Some(LoadPlan {
        daily_tss: plan_loads.to_vec(),
        predicted_states: predicted,
        goal,
        success,
        score,
        messages,
        form_start,
        form_end,
        chronic_ref,
        mean_plan_load: mean_plan,
        load_frac,
        projected_acwr,
    })
}

/// Legacy stub retained for API compatibility (element-wise product).
#[deprecated(note = "use optimize_load_plan for goal-conditioned multi-day planning")]
pub fn optimize_load(parameters: &[f64], data: &[f64]) -> Vec<f64> {
    parameters.iter().zip(data.iter()).map(|(p, d)| p * d).collect()
}

#[allow(dead_code)]
pub fn apply_plan_day(state: PulseResponseState, load: f64, params: &PulseResponseParams) -> PulseResponseState {
    step_pulse_response(state, load, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::generate_demo_daily_loads;

    fn base_history() -> Vec<f64> {
        let mut loads = generate_demo_daily_loads(28, 60.0, 15.0);
        for l in loads.iter_mut().skip(21) {
            *l = 120.0;
        }
        loads
    }

    #[test]
    fn recovery_is_active_not_pure_rest() {
        let hist = base_history();
        let params = PulseResponseParams::pmc_defaults();
        let thr = OptimizationThresholds {
            horizon_days: 3,
            ..Default::default()
        };
        let plan = optimize_load_plan(&hist, &params, LoadGoal::Recovery, &thr).unwrap();
        assert!(plan.success, "messages: {:?}", plan.messages);
        assert!(
            plan.load_frac >= thr.recovery_load_frac_min - 1e-9,
            "should not be pure rest: load_frac {} days {:?}",
            plan.load_frac,
            plan.daily_tss
        );
        assert!(
            plan.load_frac <= thr.recovery_load_frac_max + 1e-9,
            "load_frac {}",
            plan.load_frac
        );
        // At least one non-rest day
        assert!(
            plan.daily_tss.iter().any(|&w| w > 1.0),
            "expected active recovery days: {:?}",
            plan.daily_tss
        );
        assert!(plan.messages.iter().any(|m| m.contains("ACWR context")));
    }

    #[test]
    fn maintenance_modulates_days_not_flat() {
        let hist = generate_demo_daily_loads(28, 70.0, 10.0);
        let params = PulseResponseParams::pmc_defaults();
        let thr = OptimizationThresholds {
            horizon_days: 3,
            ..Default::default()
        };
        let plan = optimize_load_plan(&hist, &params, LoadGoal::Maintenance, &thr).unwrap();
        assert!(plan.success, "messages: {:?}", plan.messages);
        assert!(
            plan.load_frac >= thr.maintenance_load_frac_lo - 1e-9
                && plan.load_frac <= thr.maintenance_load_frac_hi + 1e-9
        );
        let c = plan.chronic_ref;
        let max_w = plan.daily_tss.iter().copied().fold(0.0, f64::max);
        let min_w = plan.daily_tss.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(
            max_w - min_w > 0.05 * c,
            "expected modulated TSS, got flat {:?}, C={}",
            plan.daily_tss,
            c
        );
        // Still avoid pure rest / extreme spikes
        for &w in &plan.daily_tss {
            assert!(
                w >= 0.15 * c - 1.0 && w <= 1.5 * c + 1.0,
                "day load {} extreme vs C {}: {:?}",
                w,
                c,
                plan.daily_tss
            );
        }
    }

    #[test]
    fn overload_elevated_with_variety() {
        let hist = generate_demo_daily_loads(28, 55.0, 8.0);
        let params = PulseResponseParams::pmc_defaults();
        let thr = OptimizationThresholds {
            horizon_days: 3,
            ..Default::default()
        };
        let plan = optimize_load_plan(&hist, &params, LoadGoal::Overload, &thr).unwrap();
        assert!(plan.success, "messages: {:?}", plan.messages);
        assert!(plan.mean_plan_load > plan.chronic_ref);
        assert!(plan.load_frac >= thr.overload_load_frac_lo - 1e-9);
        // Not all identical max days
        let max_w = plan.daily_tss.iter().copied().fold(0.0, f64::max);
        let min_w = plan.daily_tss.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(
            max_w - min_w > 1.0 || plan.daily_tss.len() == 1,
            "prefer some variety: {:?}",
            plan.daily_tss
        );
    }

    #[test]
    fn acwr_context_does_not_hard_fail_success() {
        let mut hist = generate_demo_daily_loads(30, 80.0, 5.0);
        for l in hist.iter_mut().skip(23) {
            *l = 150.0;
        }
        let params = PulseResponseParams::pmc_defaults();
        let thr = OptimizationThresholds {
            horizon_days: 3,
            ..Default::default()
        };
        let plan = optimize_load_plan(&hist, &params, LoadGoal::Recovery, &thr).unwrap();
        assert!(plan.success, "messages: {:?}", plan.messages);
        assert!(plan.messages.iter().any(|m| m.contains("ACWR context")));
    }

    #[test]
    fn insufficient_history_errors() {
        let params = PulseResponseParams::pmc_defaults();
        let thr = OptimizationThresholds::default();
        let err = optimize_load_plan(&[10.0, 20.0], &params, LoadGoal::Maintenance, &thr);
        assert!(matches!(err, Err(LoadSymError::InsufficientData(_))));
    }

    #[test]
    fn long_horizon_beam_completes() {
        // 7^10 is huge — must use beam path and still return a plan quickly.
        let hist = generate_demo_daily_loads(30, 70.0, 10.0);
        let params = PulseResponseParams::pmc_defaults();
        let thr = OptimizationThresholds {
            horizon_days: 10,
            ..Default::default()
        };
        let plan = optimize_load_plan(&hist, &params, LoadGoal::Maintenance, &thr).unwrap();
        assert_eq!(plan.daily_tss.len(), 10);
        assert!(plan.success, "messages: {:?}", plan.messages);
    }

    #[test]
    #[allow(deprecated)]
    fn legacy_optimize_load_still_works() {
        let out = optimize_load(&[2.0, 3.0], &[4.0, 5.0]);
        assert_eq!(out, vec![8.0, 15.0]);
    }
}
