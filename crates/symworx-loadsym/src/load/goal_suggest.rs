// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Suggest a multi-day [`LoadGoal`] from current readiness (form / fatigue / ACLi).
//!
//! This is a **soft default** for planning UIs: score recovery / maintenance /
//! overload from pulse-response state and optional ACWR, then pick the best.
//! Thresholds get close; the athlete still overrides with an explicit goal.

use super::{
    optimization::LoadGoal,
    pulse_response::PulseResponseState,
};

/// Tunable bands for [`suggest_load_goal`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoalSuggestParams {
    /// Form (SLBi) at/below this strongly favors recovery (default −20).
    pub form_recovery: f64,
    /// Form at/above this favors overload when ACLi is not elevated (default +10).
    pub form_overload: f64,
    /// Mid-band half-width around 0 for maintenance preference (default 12).
    pub form_maintain_half: f64,
    /// Fatigue/fitness ratio above this favors recovery (default 1.15).
    pub fatigue_fitness_hi: f64,
    /// Fatigue/fitness ratio below this favors overload when form is ok (default 0.95).
    pub fatigue_fitness_lo: f64,
    /// ACLi at/above this soft-pushes recovery and penalizes overload (default 1.30).
    pub acwr_high: f64,
    /// ACLi at/below this soft-pushes overload when form is positive (default 0.80).
    pub acwr_low: f64,
}

impl Default for GoalSuggestParams {
    fn default() -> Self {
        Self {
            form_recovery: -20.0,
            form_overload: 10.0,
            form_maintain_half: 12.0,
            fatigue_fitness_hi: 1.15,
            fatigue_fitness_lo: 0.95,
            acwr_high: 1.30,
            acwr_low: 0.80,
        }
    }
}

/// Result of scoring the three planning goals from current readiness.
#[derive(Debug, Clone, PartialEq)]
pub struct GoalSuggestion {
    /// Recommended default goal (highest score).
    pub goal: LoadGoal,
    /// Soft confidence in \[0, 1\] from score separation.
    pub confidence: f64,
    /// Per-goal scores (higher is better). Order: Recovery, Maintenance, Overload.
    pub scores: [(LoadGoal, f64); 3],
    /// Human-readable notes for TUI / CLI.
    pub reasons: Vec<String>,
}

/// Score recovery / maintenance / overload from form, fatigue, and optional ACLi.
///
/// Inputs:
/// - `state.form` — SLBi-like readiness (fitness − fatigue under PMC defaults)
/// - `state.fatigue` / `state.fitness` — STSLi / LTSLi for ratio soft terms
/// - `acwr` — optional acute:chronic ratio (advisory only)
///
/// Does **not** run [`super::optimize_load_plan`]; it only chooses a default goal.
pub fn suggest_load_goal(
    state: &PulseResponseState,
    acwr: Option<f64>,
    params: &GoalSuggestParams,
) -> GoalSuggestion {
    let form = state.form;
    let fitness = state.fitness.max(1e-6);
    let fatigue = state.fatigue.max(0.0);
    let ff_ratio = fatigue / fitness;

    // Base scores from form (piecewise preference peaks)
    let mut rec = form_score_recovery(form, params);
    let mut mnt = form_score_maintain(form, params);
    let mut ovr = form_score_overload(form, params);

    // Fatigue relative to fitness
    if ff_ratio >= params.fatigue_fitness_hi {
        let excess = (ff_ratio - params.fatigue_fitness_hi).min(0.5);
        rec += 0.35 + 0.5 * excess;
        ovr -= 0.40 + 0.6 * excess;
        mnt -= 0.05;
    } else if ff_ratio <= params.fatigue_fitness_lo {
        let slack = (params.fatigue_fitness_lo - ff_ratio).min(0.4);
        ovr += 0.20 + 0.4 * slack;
        rec -= 0.15;
    }

    // ACLi soft terms — elevated ACLi hard-caps overload even when form is fresh.
    if let Some(a) = acwr {
        if a.is_finite() {
            if a >= params.acwr_high {
                let excess = (a - params.acwr_high).min(0.8);
                rec += 0.55 + 0.6 * excess;
                // Strong penalty: form-based overload peak is ~1.0–1.35; wipe it.
                ovr -= 1.10 + 0.9 * excess;
                mnt += 0.25 + 0.2 * excess;
            } else if a <= params.acwr_low {
                let slack = (params.acwr_low - a).min(0.5);
                ovr += 0.25 + 0.35 * slack;
                rec -= 0.10;
            }
        }
    }

    // Floor scores so ranking stays well-defined
    rec = rec.max(0.0);
    mnt = mnt.max(0.0);
    ovr = ovr.max(0.0);

    let scores = [
        (LoadGoal::Recovery, rec),
        (LoadGoal::Maintenance, mnt),
        (LoadGoal::Overload, ovr),
    ];
    let mut best = scores[0];
    for &s in &scores[1..] {
        if s.1 > best.1 {
            best = s;
        }
    }

    let mut ordered = scores;
    ordered.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let gap = ordered[0].1 - ordered[1].1;
    let confidence = (gap / 1.2).clamp(0.15, 1.0);

    let mut reasons = Vec::new();
    reasons.push(format!(
        "form(SLBi)={:+.1}  LTSLi={:.0}  STSLi={:.0}  fatigue/fitness={:.2}",
        form, state.fitness, state.fatigue, ff_ratio
    ));
    if let Some(a) = acwr {
        reasons.push(format!("ACLi={:.2} (soft context)", a));
    } else {
        reasons.push("ACLi unavailable (form/fatigue only)".into());
    }
    reasons.push(format!(
        "scores: recovery={:.2}  maintain={:.2}  overload={:.2}",
        rec, mnt, ovr
    ));
    reasons.push(format!(
        "suggested {} (confidence {:.0}%)",
        best.0.as_str(),
        confidence * 100.0
    ));

    GoalSuggestion {
        goal: best.0,
        confidence,
        scores,
        reasons,
    }
}

/// Recovery peak when form is deeply negative.
fn form_score_recovery(form: f64, p: &GoalSuggestParams) -> f64 {
    if form <= p.form_recovery {
        // deeper negative → higher
        let depth = (p.form_recovery - form).min(40.0) / 40.0;
        1.0 + 0.5 * depth
    } else if form < 0.0 {
        // taper from recovery threshold toward 0
        let t = form / p.form_recovery; // form negative, recovery negative → t in (0,1]
        0.55 * t.clamp(0.0, 1.0)
    } else {
        0.15 * (-form / 30.0).exp().clamp(0.0, 1.0)
    }
}

/// Maintenance peaks near form ≈ 0.
fn form_score_maintain(form: f64, p: &GoalSuggestParams) -> f64 {
    let half = p.form_maintain_half.max(1.0);
    let d = (form / half).abs();
    if d <= 1.0 {
        1.0 - 0.35 * d
    } else if d <= 2.5 {
        0.65 - 0.25 * (d - 1.0)
    } else {
        0.15
    }
}

/// Overload peaks when form is clearly positive.
fn form_score_overload(form: f64, p: &GoalSuggestParams) -> f64 {
    if form >= p.form_overload {
        let head = ((form - p.form_overload) / 25.0).min(1.0);
        1.0 + 0.35 * head
    } else if form > 0.0 {
        form / p.form_overload.max(1.0) * 0.70
    } else {
        // negative form: low overload score
        0.08 * (1.0 + form / 40.0).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::PulseResponseParams;

    fn state(fitness: f64, fatigue: f64) -> PulseResponseState {
        let p = PulseResponseParams::pmc_defaults();
        PulseResponseState {
            fitness,
            fatigue,
            performance: 0.0,
            form: fitness - fatigue,
        }
        .finalize(&p)
    }

    #[test]
    fn deep_fatigue_high_acwr_suggests_recovery() {
        // form = −35, elevated ACLi
        let s = state(80.0, 115.0);
        assert!((s.form + 35.0).abs() < 1e-9);
        let g = suggest_load_goal(&s, Some(1.40), &GoalSuggestParams::default());
        assert_eq!(g.goal, LoadGoal::Recovery, "reasons: {:?}", g.reasons);
    }

    #[test]
    fn fresh_low_acwr_suggests_overload() {
        let s = state(70.0, 50.0); // form +20
        let g = suggest_load_goal(&s, Some(0.90), &GoalSuggestParams::default());
        assert_eq!(g.goal, LoadGoal::Overload, "reasons: {:?}", g.reasons);
    }

    #[test]
    fn near_neutral_suggests_maintenance() {
        let s = state(75.0, 80.0); // form −5
        let g = suggest_load_goal(&s, Some(1.00), &GoalSuggestParams::default());
        assert_eq!(g.goal, LoadGoal::Maintenance, "reasons: {:?}", g.reasons);
    }

    #[test]
    fn fresh_but_high_acwr_avoids_overload() {
        let s = state(90.0, 70.0); // form +20
        let g = suggest_load_goal(&s, Some(1.50), &GoalSuggestParams::default());
        assert_ne!(
            g.goal,
            LoadGoal::Overload,
            "high ACLi should block overload: {:?}",
            g.reasons
        );
        assert!(
            matches!(g.goal, LoadGoal::Recovery | LoadGoal::Maintenance),
            "got {:?}",
            g.goal
        );
    }

    #[test]
    fn no_acwr_still_uses_form() {
        let s = state(60.0, 95.0); // form −35
        let g = suggest_load_goal(&s, None, &GoalSuggestParams::default());
        assert_eq!(g.goal, LoadGoal::Recovery, "reasons: {:?}", g.reasons);
        assert!(g.reasons.iter().any(|r| r.contains("unavailable")));
    }
}
