// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Pulse-response (fitness–fatigue / Banister impulse-response) model.
//!
//! Models the athlete as a pair of first-order linear filters driven by daily
//! training load \(w_t\) (typically TSS):
//!
//! \[
//! \begin{aligned}
//! g_t &= g_{t-1}\,e^{-1/\tau_g} + w_t \\
//! h_t &= h_{t-1}\,e^{-1/\tau_h} + w_t \\
//! p_t &= p_0 + k_g\,g_t - k_h\,h_t
//! \end{aligned}
//! \]
//!
//! - \(g\) = fitness (slow positive adaptation)
//! - \(h\) = fatigue (fast negative effect)
//! - \(p\) = performance / readiness proxy
//! - `form` = \(g - h\) (TSB-like when \(k_g = k_h = 1\))
//!
//! The PMC (TrainingPeaks-style CTL / ATL / TSB) interpretation uses unit gains
//! via [`PulseResponseParams::pmc_defaults`]. Full Banister gains use
//! [`PulseResponseParams::banister_defaults`].
//!
//! Discrete recursion is preferred for daily catalog series. A continuous ODE
//! helper is available for teaching / dynamics demos (Kim-style LTI systems).

use crate::error::{
    LoadSymError,
    Result,
};

/// Discrete update rule for the two compartments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PulseUpdateRule {
    /// Banister impulse-response: \(x_t = x_{t-1} e^{-1/\tau} + w_t\).
    ///
    /// With equal gains, \(g - h\) is independent of \(w\) on the load day
    /// (impulse cancels). Use \(k_h > k_g\) so performance responds to load,
    /// or prefer [`PulseUpdateRule::PmcEwma`] for CTL/ATL/TSB planning.
    Banister,
    /// TrainingPeaks-style EWMA: \(x_t = x_{t-1} + (w_t - x_{t-1})/\tau\).
    ///
    /// TSB = CTL − ATL depends on daily load (preferred for planning).
    PmcEwma,
}

/// Parameters for the two-component pulse-response model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PulseResponseParams {
    /// Fitness time constant (days). Typical: ~42.
    pub tau_fitness: f64,
    /// Fatigue time constant (days). Typical: ~7.
    pub tau_fatigue: f64,
    /// Fitness gain \(k_g\).
    pub k_fitness: f64,
    /// Fatigue gain \(k_h\) (usually \(\ge k_g\) so acute fatigue dominates).
    pub k_fatigue: f64,
    /// Baseline performance offset \(p_0\).
    pub p0: f64,
    /// Compartment update rule.
    pub update: PulseUpdateRule,
}

impl Default for PulseResponseParams {
    fn default() -> Self {
        Self::pmc_defaults()
    }
}

impl PulseResponseParams {
    /// PMC-style defaults: τ_fitness=42, τ_fatigue=7, unit gains, EWMA updates → CTL/ATL/TSB.
    pub fn pmc_defaults() -> Self {
        Self {
            tau_fitness: 42.0,
            tau_fatigue: 7.0,
            k_fitness: 1.0,
            k_fatigue: 1.0,
            p0: 0.0,
            update: PulseUpdateRule::PmcEwma,
        }
    }

    /// Banister-style defaults with larger fatigue gain (short-term performance dip).
    pub fn banister_defaults() -> Self {
        Self {
            tau_fitness: 45.0,
            tau_fatigue: 15.0,
            k_fitness: 1.0,
            k_fatigue: 2.0,
            p0: 0.0,
            update: PulseUpdateRule::Banister,
        }
    }

    /// Decay factor for fitness: \(e^{-1/\tau_g}\) (Banister only).
    #[inline]
    pub fn fitness_decay(&self) -> f64 {
        (-1.0 / self.tau_fitness.max(1e-9)).exp()
    }

    /// Decay factor for fatigue: \(e^{-1/\tau_h}\) (Banister only).
    #[inline]
    pub fn fatigue_decay(&self) -> f64 {
        (-1.0 / self.tau_fatigue.max(1e-9)).exp()
    }

    /// Validate time constants and gains are finite and positive where required.
    pub fn validate(&self) -> Result<()> {
        if !(self.tau_fitness.is_finite() && self.tau_fitness > 0.0) {
            return Err(LoadSymError::InvalidParameter(
                "tau_fitness must be finite and > 0".into(),
            ));
        }
        if !(self.tau_fatigue.is_finite() && self.tau_fatigue > 0.0) {
            return Err(LoadSymError::InvalidParameter(
                "tau_fatigue must be finite and > 0".into(),
            ));
        }
        if !self.k_fitness.is_finite() || !self.k_fatigue.is_finite() || !self.p0.is_finite() {
            return Err(LoadSymError::InvalidParameter(
                "gains and p0 must be finite".into(),
            ));
        }
        Ok(())
    }
}

/// Instantaneous model state after a load impulse (or at rest).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PulseResponseState {
    /// Fitness \(g\) (CTL-like under PMC params).
    pub fitness: f64,
    /// Fatigue \(h\) (ATL-like under PMC params).
    pub fatigue: f64,
    /// Performance \(p = p_0 + k_g g - k_h h\).
    pub performance: f64,
    /// Form proxy \(g - h\) (TSB-like under unit gains).
    pub form: f64,
}

impl PulseResponseState {
    /// Zero fitness/fatigue at baseline performance.
    pub fn zero(params: &PulseResponseParams) -> Self {
        Self {
            fitness: 0.0,
            fatigue: 0.0,
            performance: params.p0,
            form: 0.0,
        }
    }

    /// Recompute `performance` and `form` from fitness/fatigue and params.
    pub fn finalize(mut self, params: &PulseResponseParams) -> Self {
        self.performance =
            params.p0 + params.k_fitness * self.fitness - params.k_fatigue * self.fatigue;
        self.form = self.fitness - self.fatigue;
        self
    }
}

/// Full time series of model states (one entry per input day).
#[derive(Debug, Clone, PartialEq)]
pub struct PulseResponseSeries {
    pub fitness: Vec<f64>,
    pub fatigue: Vec<f64>,
    pub performance: Vec<f64>,
    pub form: Vec<f64>,
}

impl PulseResponseSeries {
    pub fn len(&self) -> usize {
        self.fitness.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fitness.is_empty()
    }

    /// Last state, if any.
    pub fn last_state(&self) -> Option<PulseResponseState> {
        let n = self.len();
        if n == 0 {
            return None;
        }
        Some(PulseResponseState {
            fitness: self.fitness[n - 1],
            fatigue: self.fatigue[n - 1],
            performance: self.performance[n - 1],
            form: self.form[n - 1],
        })
    }

    fn push_state(&mut self, s: PulseResponseState) {
        self.fitness.push(s.fitness);
        self.fatigue.push(s.fatigue);
        self.performance.push(s.performance);
        self.form.push(s.form);
    }

    fn empty() -> Self {
        Self {
            fitness: Vec::new(),
            fatigue: Vec::new(),
            performance: Vec::new(),
            form: Vec::new(),
        }
    }

    fn with_capacity(n: usize) -> Self {
        Self {
            fitness: Vec::with_capacity(n),
            fatigue: Vec::with_capacity(n),
            performance: Vec::with_capacity(n),
            form: Vec::with_capacity(n),
        }
    }
}

/// Apply one day of load to the current state (before → after).
///
/// State on entry is the *previous* day's end state (or zero).
pub fn step_pulse_response(
    state: PulseResponseState,
    load: f64,
    params: &PulseResponseParams,
) -> PulseResponseState {
    let w = if load.is_finite() && load > 0.0 {
        load
    } else {
        0.0
    };

    let (fitness, fatigue) = match params.update {
        PulseUpdateRule::Banister => {
            let dg = params.fitness_decay();
            let dh = params.fatigue_decay();
            (state.fitness * dg + w, state.fatigue * dh + w)
        }
        PulseUpdateRule::PmcEwma => {
            // x ← x + (w − x) / τ   (TrainingPeaks CTL/ATL style)
            let tg = params.tau_fitness.max(1e-9);
            let th = params.tau_fatigue.max(1e-9);
            (
                state.fitness + (w - state.fitness) / tg,
                state.fatigue + (w - state.fatigue) / th,
            )
        }
    };

    PulseResponseState {
        fitness,
        fatigue,
        performance: 0.0,
        form: 0.0,
    }
    .finalize(params)
}

/// Forward-simulate over a daily load series (oldest → newest).
///
/// Returns one state per input day. If `initial` is `None`, starts from zero.
pub fn simulate_pulse_response(
    daily_loads: &[f64],
    params: &PulseResponseParams,
    initial: Option<PulseResponseState>,
) -> Result<PulseResponseSeries> {
    params.validate()?;
    let mut state = initial.unwrap_or_else(|| PulseResponseState::zero(params));
    let mut out = PulseResponseSeries::with_capacity(daily_loads.len());
    for &w in daily_loads {
        state = step_pulse_response(state, w, params);
        out.push_state(state);
    }
    Ok(out)
}

/// Open-loop forecast: continue from `state` with the given future daily loads.
pub fn forecast_pulse_response(
    state: PulseResponseState,
    future_loads: &[f64],
    params: &PulseResponseParams,
) -> Result<PulseResponseSeries> {
    params.validate()?;
    let mut s = state;
    let mut out = PulseResponseSeries::with_capacity(future_loads.len());
    for &w in future_loads {
        s = step_pulse_response(s, w, params);
        out.push_state(s);
    }
    Ok(out)
}

/// Forecast under a constant daily load for `days` days.
pub fn forecast_with_constant_load(
    state: PulseResponseState,
    daily_load: f64,
    params: &PulseResponseParams,
    days: usize,
) -> Result<PulseResponseSeries> {
    let future = vec![daily_load; days];
    forecast_pulse_response(state, &future, params)
}

/// Estimate days until `form >= target_form` under a constant assumed daily load.
///
/// Returns `None` if not reached within `max_days`. Useful for recovery planning.
pub fn estimate_recovery_days(
    state: PulseResponseState,
    params: &PulseResponseParams,
    target_form: f64,
    assumed_daily_load: f64,
    max_days: usize,
) -> Result<Option<usize>> {
    params.validate()?;
    if state.form >= target_form {
        return Ok(Some(0));
    }
    let mut s = state;
    for d in 1..=max_days {
        s = step_pulse_response(s, assumed_daily_load, params);
        if s.form >= target_form {
            return Ok(Some(d));
        }
    }
    Ok(None)
}

/// Unit-impulse response of form (and components) for `horizon` days after a unit load day.
///
/// Day 0 applies load=1; subsequent days apply load=0. Useful for teaching LTI
/// impulse-response behaviour (Kim / Banister).
pub fn unit_impulse_response(
    params: &PulseResponseParams,
    horizon: usize,
) -> Result<PulseResponseSeries> {
    params.validate()?;
    if horizon == 0 {
        return Ok(PulseResponseSeries::empty());
    }
    let mut loads = vec![0.0; horizon];
    loads[0] = 1.0;
    simulate_pulse_response(&loads, params, None)
}

/// Continuous-time ODE integration of the same two-compartment model.
///
/// \(\dot g = -g/\tau_g + w(t)\), \(\dot h = -h/\tau_h + w(t)\).
///
/// `load_fn(t)` supplies instantaneous load rate (load units per day).
/// Uses fixed-step RK4 from `symworx_math`. Intended for demos / analysis,
/// not the daily catalog path.
pub fn simulate_pulse_response_continuous<F>(
    load_fn: F,
    params: &PulseResponseParams,
    t_span: (f64, f64),
    y0: PulseResponseState,
    dt: f64,
) -> Result<(Vec<f64>, Vec<PulseResponseState>)>
where
    F: Fn(f64) -> f64,
{
    use ndarray::Array1;
    use symworx_core::math::integration::rk4_integrate;

    params.validate()?;
    if !(dt.is_finite() && dt > 0.0) {
        return Err(LoadSymError::InvalidParameter(
            "dt must be finite and > 0".into(),
        ));
    }

    let tau_g = params.tau_fitness.max(1e-9);
    let tau_h = params.tau_fatigue.max(1e-9);
    let p0 = params.p0;
    let kg = params.k_fitness;
    let kh = params.k_fatigue;

    let f = |t: f64, y: &Array1<f64>| -> Array1<f64> {
        let w = load_fn(t);
        let g = y[0];
        let h = y[1];
        Array1::from(vec![-g / tau_g + w, -h / tau_h + w])
    };

    let y0_arr = Array1::from(vec![y0.fitness, y0.fatigue]);
    let (times, states) = rk4_integrate(f, t_span, y0_arr, dt);

    let out: Vec<PulseResponseState> = states
        .into_iter()
        .map(|y| {
            PulseResponseState {
                fitness: y[0],
                fatigue: y[1],
                performance: 0.0,
                form: 0.0,
            }
            .finalize(&PulseResponseParams {
                tau_fitness: tau_g,
                tau_fatigue: tau_h,
                k_fitness: kg,
                k_fatigue: kh,
                p0,
                update: params.update,
            })
        })
        .collect();

    Ok((times, out))
}

/// Convenience aliases matching PMC naming when using unit-gain params.
impl PulseResponseState {
    /// CTL-like (fitness).
    #[inline]
    pub fn ctl(&self) -> f64 {
        self.fitness
    }
    /// ATL-like (fatigue).
    #[inline]
    pub fn atl(&self) -> f64 {
        self.fatigue
    }
    /// TSB-like (form = fitness − fatigue).
    #[inline]
    pub fn tsb(&self) -> f64 {
        self.form
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_load_approaches_steady_state_pmc() {
        let params = PulseResponseParams::pmc_defaults();
        let loads = vec![100.0; 300];
        let series = simulate_pulse_response(&loads, &params, None).unwrap();
        let last = series.last_state().unwrap();
        // EWMA steady state is the constant load itself
        assert!((last.fitness - 100.0).abs() < 1.0);
        assert!((last.fatigue - 100.0).abs() < 1.0);
        assert!(last.form.abs() < 1.0);
    }

    #[test]
    fn constant_load_approaches_steady_state_banister() {
        let params = PulseResponseParams::banister_defaults();
        let loads = vec![100.0; 300];
        let series = simulate_pulse_response(&loads, &params, None).unwrap();
        let last = series.last_state().unwrap();
        let g_ss = 100.0 / (1.0 - params.fitness_decay());
        let h_ss = 100.0 / (1.0 - params.fatigue_decay());
        assert!((last.fitness - g_ss).abs() / g_ss < 0.05);
        assert!((last.fatigue - h_ss).abs() / h_ss < 0.05);
    }

    #[test]
    fn rest_after_hard_block_raises_form() {
        let params = PulseResponseParams::pmc_defaults();
        let mut loads = vec![80.0; 28];
        // hard week
        for l in loads.iter_mut().skip(21) {
            *l = 150.0;
        }
        let hist = simulate_pulse_response(&loads, &params, None).unwrap();
        let after_hard = hist.last_state().unwrap();
        let rest = forecast_with_constant_load(after_hard, 0.0, &params, 7).unwrap();
        let after_rest = rest.last_state().unwrap();
        // Fatigue decays faster → form should rise during rest
        assert!(after_rest.form > after_hard.form);
        assert!(after_rest.fatigue < after_hard.fatigue);
    }

    #[test]
    fn zero_load_decays_to_baseline() {
        let params = PulseResponseParams::pmc_defaults();
        let start = PulseResponseState {
            fitness: 50.0,
            fatigue: 40.0,
            performance: 0.0,
            form: 0.0,
        }
        .finalize(&params);
        let series = forecast_with_constant_load(start, 0.0, &params, 200).unwrap();
        let last = series.last_state().unwrap();
        assert!(last.fitness < 1.0);
        assert!(last.fatigue < 0.1);
        assert!((last.performance - params.p0).abs() < 1.0);
    }

    #[test]
    fn unit_impulse_decays_banister() {
        let params = PulseResponseParams::banister_defaults();
        let ir = unit_impulse_response(&params, 30).unwrap();
        assert_eq!(ir.len(), 30);
        assert!((ir.fitness[0] - 1.0).abs() < 1e-12);
        // Monotone decay after impulse for both compartments under rest
        assert!(ir.fitness[10] < ir.fitness[1]);
        assert!(ir.fatigue[5] < ir.fatigue[0]);
        // Fatigue decays faster
        assert!(ir.fatigue[10] < ir.fitness[10]);
    }

    #[test]
    fn pmc_hard_day_lowers_form() {
        let params = PulseResponseParams::pmc_defaults();
        // Build mild chronic base
        let base = vec![50.0; 40];
        let hist = simulate_pulse_response(&base, &params, None).unwrap();
        let s = hist.last_state().unwrap();
        let after_hard = step_pulse_response(s, 150.0, &params);
        assert!(
            after_hard.form < s.form,
            "hard day should drop TSB: {} → {}",
            s.form,
            after_hard.form
        );
    }

    #[test]
    fn recovery_days_finds_horizon() {
        let params = PulseResponseParams::pmc_defaults();
        // Build a fatigued state
        let loads = vec![120.0; 14];
        let hist = simulate_pulse_response(&loads, &params, None).unwrap();
        let s = hist.last_state().unwrap();
        let target = s.form + 5.0; // modest form improvement
        let days = estimate_recovery_days(s, &params, target, 20.0, 60)
            .unwrap()
            .expect("should reach target");
        assert!(days > 0 && days <= 60);
    }

    #[test]
    fn invalid_tau_errors() {
        let mut p = PulseResponseParams::pmc_defaults();
        p.tau_fitness = 0.0;
        assert!(matches!(
            simulate_pulse_response(&[1.0], &p, None),
            Err(LoadSymError::InvalidParameter(_))
        ));
    }

    #[test]
    fn continuous_smoke() {
        let params = PulseResponseParams::pmc_defaults();
        let y0 = PulseResponseState::zero(&params);
        let (times, states) =
            simulate_pulse_response_continuous(|_| 10.0, &params, (0.0, 5.0), y0, 0.25).unwrap();
        assert!(times.len() > 2);
        assert_eq!(times.len(), states.len());
        assert!(states.last().unwrap().fitness > 0.0);
    }
}
