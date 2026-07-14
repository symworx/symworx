// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Multi-day load planning on top of the pulse-response model.
//!
//! Given recent daily loads (e.g. TSS), choose a short horizon plan
//! (default 3 days) aimed at one of:
//! - [`LoadGoal::Overload`] — productive progressive load (controlled form dip)
//! - [`LoadGoal::Maintenance`] — hold form within a relative tolerance (~20%)
//! - [`LoadGoal::Recovery`] — clear fatigue / raise form with light days
//!
//! Search uses a discrete template library (rest / easy / steady / hard / long)
//! scaled to chronic load — no external solver dependency.

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
    /// Raise acute load with a controlled short-term form dip.
    Overload,
    /// Keep form near current level (relative drift ≤ threshold).
    Maintenance,
    /// Prioritize form recovery with reduced load.
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

/// Success / failure thresholds for [`optimize_load_plan`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptimizationThresholds {
    /// Max relative form drift for Maintenance success (default 0.20 = 20%).
    pub maintenance_form_rel_tol: f64,
    /// Minimum fraction of desired form gain for Recovery success (default 0.80).
    pub recovery_min_fraction_of_gain: f64,
    /// Below this fraction of desired gain → Recovery hard failure (default 0.20).
    pub recovery_fail_fraction_of_gain: f64,
    /// Max overshoot of intended form dip for Overload (default 1.20 = 120%).
    pub overload_max_overshoot: f64,
    /// Target form dip for Overload (TSB units; positive number meaning how far form drops).
    pub overload_target_form_dip: f64,
    /// Hard ACWR safety cap on projected history+plan (default 1.5).
    pub acwr_hard_cap: f64,
    /// Planning horizon in days (default 3).
    pub horizon_days: usize,
    /// Max daily load as multiple of recent max (default 1.5).
    pub max_load_factor: f64,
    /// Allow plans that breach ACWR hard cap (default false).
    pub allow_aggressive: bool,
    /// Minimum history days preferred for chronic estimate (default 14).
    pub min_history_days: usize,
}

impl Default for OptimizationThresholds {
    fn default() -> Self {
        Self {
            maintenance_form_rel_tol: 0.20,
            recovery_min_fraction_of_gain: 0.80,
            recovery_fail_fraction_of_gain: 0.20,
            overload_max_overshoot: 1.20,
            overload_target_form_dip: 15.0,
            acwr_hard_cap: 1.5,
            horizon_days: 3,
            max_load_factor: 1.5,
            allow_aggressive: false,
            min_history_days: 7,
        }
    }
}

/// Result of multi-day load optimization.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadPlan {
    /// Recommended daily TSS (or load units) for each horizon day.
    pub daily_tss: Vec<f64>,
    /// Predicted pulse-response states over the plan horizon only.
    pub predicted_states: PulseResponseSeries,
    pub goal: LoadGoal,
    /// Whether the plan meets success criteria (and safety constraints).
    pub success: bool,
    /// Lower is better (used for ranking candidate sequences).
    pub score: f64,
    /// Human-readable notes (goal assessment, safety, etc.).
    pub messages: Vec<String>,
    /// Form at start of plan (after history).
    pub form_start: f64,
    /// Form at end of plan.
    pub form_end: f64,
    /// Chronic load estimate used to scale templates.
    pub chronic_ref: f64,
}

/// Day templates as fractions of chronic reference load.
const TEMPLATE_FRACS: [f64; 5] = [0.0, 0.4, 1.0, 1.3, 1.5]; // rest, easy, steady, hard, long

/// Optimize a short multi-day load plan for the given goal.
///
/// `history_loads` should be oldest → newest daily loads (TSS). Requires at least
/// a few days of history for a meaningful chronic reference.
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
    if thresholds.horizon_days == 0 || thresholds.horizon_days > 7 {
        return Err(LoadSymError::InvalidParameter(
            "horizon_days must be in 1..=7".into(),
        ));
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
    let recent_max = history_loads
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(chronic_ref);
    let w_max = (recent_max * thresholds.max_load_factor).max(1.0);

    let templates: Vec<f64> = TEMPLATE_FRACS
        .iter()
        .map(|f| (f * chronic_ref).clamp(0.0, w_max))
        .collect();

    let h = thresholds.horizon_days;
    let mut best: Option<LoadPlan> = None;

    // Enumerate all template sequences of length h (5^h; h<=7 → manageable for small h;
    // for h=5: 3125, h=6: 15625, h=7: 78125 — fine for offline planning).
    let n_t = templates.len();
    let total = n_t.pow(h as u32);
    for idx in 0..total {
        let mut seq = Vec::with_capacity(h);
        let mut rem = idx;
        for _ in 0..h {
            seq.push(templates[rem % n_t]);
            rem /= n_t;
        }

        if let Some(plan) = evaluate_candidate(
            history_loads,
            &seq,
            start,
            params,
            goal,
            thresholds,
            chronic_ref,
        ) {
            let replace = match &best {
                None => true,
                Some(b) => rank_key(&plan) < rank_key(b),
            };
            if replace {
                best = Some(plan);
            }
        }
    }

    best.ok_or_else(|| {
        LoadSymError::InvalidValue("no valid load plan candidates under constraints".into())
    })
}

/// Prefer successful plans, then lower score.
fn rank_key(p: &LoadPlan) -> (u8, i64) {
    let fail = if p.success { 0u8 } else { 1u8 };
    // quantize score for Ord
    let s = (p.score * 1e6) as i64;
    (fail, s)
}

fn chronic_reference(history: &[f64]) -> f64 {
    let n = history.len();
    let window = n.clamp(1, 28);
    let slice = &history[n - window..];
    let mean = slice.iter().sum::<f64>() / slice.len() as f64;
    if mean.is_finite() && mean > 1.0 {
        mean
    } else {
        50.0 // fallback so templates are non-degenerate
    }
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

    // Safety: projected ACWR on history + plan
    let mut extended: Vec<f64> = history.to_vec();
    extended.extend_from_slice(plan_loads);
    let acwr_breach = match compute_acute_chronic(&extended, 7, 28) {
        Ok(s) => s.acwr > thr.acwr_hard_cap,
        Err(_) => false, // not enough for full chronic — skip hard check
    };
    if acwr_breach && !thr.allow_aggressive {
        return Some(LoadPlan {
            daily_tss: plan_loads.to_vec(),
            predicted_states: predicted,
            goal,
            success: false,
            score: 1e9,
            messages: vec![format!(
                "FAIL safety: projected ACWR exceeds hard cap {:.2}",
                thr.acwr_hard_cap
            )],
            form_start,
            form_end,
            chronic_ref,
        });
    }

    let mean_plan = plan_loads.iter().sum::<f64>() / plan_loads.len().max(1) as f64;
    let mut messages = Vec::new();
    let (success, score) = match goal {
        LoadGoal::Maintenance => {
            let denom = form_start.abs().max(10.0); // avoid tiny-denominator blowup
            let rel = (form_end - form_start).abs() / denom;
            let ok = rel <= thr.maintenance_form_rel_tol;
            // Prefer form stability + loads near chronic
            let load_dev = (mean_plan - chronic_ref).abs() / chronic_ref.max(1.0);
            let score = rel + 0.25 * load_dev;
            if ok {
                messages.push(format!(
                    "OK maintenance: form drift {:.1}% ≤ {:.0}%",
                    rel * 100.0,
                    thr.maintenance_form_rel_tol * 100.0
                ));
            } else {
                messages.push(format!(
                    "FAIL maintenance: form drift {:.1}% > {:.0}%",
                    rel * 100.0,
                    thr.maintenance_form_rel_tol * 100.0
                ));
            }
            (ok, score)
        }
        LoadGoal::Recovery => {
            // Desired: raise form; ideal is rest-ish trajectory target
            let rest_traj =
                forecast_pulse_response(start, &vec![0.0; plan_loads.len()], params).ok()?;
            let form_rest = rest_traj.last_state()?.form;
            let desired_gain = (form_rest - form_start).max(1.0);
            let gain = form_end - form_start;
            let frac = gain / desired_gain;
            let ok = gain > 0.0 && frac >= thr.recovery_min_fraction_of_gain;
            // Hard fail band for messaging (still ranked by score)
            if frac < thr.recovery_fail_fraction_of_gain {
                messages.push(format!(
                    "FAIL recovery: form gain only {:.0}% of rest-trajectory target",
                    frac * 100.0
                ));
            } else if ok {
                messages.push(format!(
                    "OK recovery: form {:+.1} ({:.0}% of rest-trajectory gain)",
                    gain,
                    frac * 100.0
                ));
            } else {
                messages.push(format!(
                    "PARTIAL recovery: form {:+.1} ({:.0}% of target; need ≥ {:.0}%)",
                    gain,
                    frac * 100.0,
                    thr.recovery_min_fraction_of_gain * 100.0
                ));
            }
            // Prefer higher form gain and lower total load
            let score = -gain + 0.01 * mean_plan;
            (ok && !acwr_breach, score)
        }
        LoadGoal::Overload => {
            let dip = form_start - form_end; // positive if form dropped
            let target = thr.overload_target_form_dip;
            let overshoot = dip > target * thr.overload_max_overshoot;
            // Success: meaningful dip into band without overshoot; mean load ≥ chronic
            let in_band = dip >= target * 0.5 && dip <= target * thr.overload_max_overshoot;
            let progressive = mean_plan >= chronic_ref * 0.95;
            let ok = in_band && progressive && !overshoot;
            if overshoot {
                messages.push(format!(
                    "FAIL overload: form dip {:.1} overshoots {:.0}% of target {:.1}",
                    dip,
                    thr.overload_max_overshoot * 100.0,
                    target
                ));
            } else if ok {
                messages.push(format!(
                    "OK overload: form dip {:.1} (target ~{:.1}), mean load {:.0}",
                    dip, target, mean_plan
                ));
            } else {
                messages.push(format!(
                    "PARTIAL overload: form dip {:.1}, mean load {:.0} (chronic {:.0})",
                    dip, mean_plan, chronic_ref
                ));
            }
            // Score: distance from target dip + prefer progressive load
            let score = (dip - target).abs() + if progressive { 0.0 } else { 20.0 };
            (ok && !acwr_breach, score)
        }
    };

    if acwr_breach {
        messages.push(format!(
            "note: projected ACWR above {:.2} (aggressive)",
            thr.acwr_hard_cap
        ));
    }

    // Secondary risk label for messaging
    if let Ok(s) = compute_acute_chronic(&extended, 7, 28) {
        messages.push(format!(
            "projected ACWR={:.2} ({})",
            s.acwr,
            classify_acwr(s.acwr).as_str()
        ));
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
    })
}

/// Legacy stub retained for API compatibility (element-wise product).
///
/// Prefer [`optimize_load_plan`] for real planning. This will be removed in a
/// future major version.
#[deprecated(note = "use optimize_load_plan for goal-conditioned multi-day planning")]
pub fn optimize_load(parameters: &[f64], data: &[f64]) -> Vec<f64> {
    parameters
        .iter()
        .zip(data.iter())
        .map(|(p, d)| p * d)
        .collect()
}

/// Single-step helper used by tests / interactive tools.
#[allow(dead_code)]
pub fn apply_plan_day(
    state: PulseResponseState,
    load: f64,
    params: &PulseResponseParams,
) -> PulseResponseState {
    step_pulse_response(state, load, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::generate_demo_daily_loads;

    fn base_history() -> Vec<f64> {
        // ~4 weeks moderate then a hard finish
        let mut loads = generate_demo_daily_loads(28, 60.0, 15.0);
        for l in loads.iter_mut().skip(21) {
            *l = 120.0;
        }
        loads
    }

    #[test]
    fn recovery_plan_reduces_load_and_raises_form() {
        let hist = base_history();
        let params = PulseResponseParams::pmc_defaults();
        let thr = OptimizationThresholds {
            horizon_days: 3,
            ..Default::default()
        };
        let plan = optimize_load_plan(&hist, &params, LoadGoal::Recovery, &thr).unwrap();
        let mean_hist: f64 = hist.iter().sum::<f64>() / hist.len() as f64;
        let mean_plan: f64 = plan.daily_tss.iter().sum::<f64>() / plan.daily_tss.len() as f64;
        assert!(
            mean_plan < mean_hist,
            "recovery mean {} should be < hist {}",
            mean_plan,
            mean_hist
        );
        assert!(
            plan.form_end >= plan.form_start,
            "form should not drop in recovery"
        );
        assert!(plan.success, "messages: {:?}", plan.messages);
    }

    #[test]
    fn maintenance_stays_within_tol() {
        let hist = generate_demo_daily_loads(28, 70.0, 10.0);
        let params = PulseResponseParams::pmc_defaults();
        let thr = OptimizationThresholds {
            horizon_days: 3,
            maintenance_form_rel_tol: 0.20,
            ..Default::default()
        };
        let plan = optimize_load_plan(&hist, &params, LoadGoal::Maintenance, &thr).unwrap();
        let denom = plan.form_start.abs().max(10.0);
        let rel = (plan.form_end - plan.form_start).abs() / denom;
        assert!(
            rel <= thr.maintenance_form_rel_tol + 1e-9,
            "drift {} messages {:?}",
            rel,
            plan.messages
        );
        assert!(plan.success);
    }

    #[test]
    fn overload_increases_acute_load() {
        let hist = generate_demo_daily_loads(28, 55.0, 8.0);
        let params = PulseResponseParams::pmc_defaults();
        let thr = OptimizationThresholds {
            horizon_days: 3,
            overload_target_form_dip: 10.0,
            allow_aggressive: true, // short demo series may spike ACWR
            ..Default::default()
        };
        let plan = optimize_load_plan(&hist, &params, LoadGoal::Overload, &thr).unwrap();
        let mean_plan: f64 = plan.daily_tss.iter().sum::<f64>() / plan.daily_tss.len() as f64;
        assert!(
            mean_plan >= plan.chronic_ref * 0.9,
            "overload mean {} chronic {}",
            mean_plan,
            plan.chronic_ref
        );
        // Form should not rise (we're stressing)
        assert!(
            plan.form_end <= plan.form_start + 1.0,
            "form_start {} form_end {}",
            plan.form_start,
            plan.form_end
        );
    }

    #[test]
    fn insufficient_history_errors() {
        let params = PulseResponseParams::pmc_defaults();
        let thr = OptimizationThresholds::default();
        let err = optimize_load_plan(&[10.0, 20.0], &params, LoadGoal::Maintenance, &thr);
        assert!(matches!(err, Err(LoadSymError::InsufficientData(_))));
    }

    #[test]
    #[allow(deprecated)]
    fn legacy_optimize_load_still_works() {
        let out = optimize_load(&[2.0, 3.0], &[4.0, 5.0]);
        assert_eq!(out, vec![8.0, 15.0]);
    }
}
