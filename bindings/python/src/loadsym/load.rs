// Copyright (c) 2026 SymWorx

use pyo3::{
    prelude::*,
    wrap_pyfunction,
};
use symworx_loadsym::load::{
    AcwrSnapshot,
    LoadGoal,
    OptimizationThresholds,
    PulseResponseParams,
    RideMetrics,
    RiskLevel,
    calculate_mechanical_load,
    calculate_physiological_load,
    // New high-value surface
    classify_acwr,
    compute_acute_chronic,
    compute_acwr_series,
    compute_ewma_acute_chronic,
    compute_monotony,
    compute_ride_metrics,
    compute_strain,
    highest_rolling,
    optimize_load,
    optimize_load_plan,
    simulate_pulse_response,
};

// ==========================================================
// Mechanical load
// ==========================================================

#[pyfunction(name = "calculate_mechanical_load")]
pub fn py_calculate_mechanical_load(
    force_data: Vec<f64>,
    velocity_data: Vec<f64>,
) -> PyResult<f64> {
    if force_data.len() != velocity_data.len() {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "force_data and velocity_data must have the same length ({} vs {})",
            force_data.len(),
            velocity_data.len()
        )));
    }
    Ok(calculate_mechanical_load(&force_data, &velocity_data))
}

// ==========================================================
// Optimization (legacy stub + multi-day plan)
// ==========================================================

#[pyfunction(name = "optimize_load")]
pub fn py_optimize_load(parameters: Vec<f64>, data: Vec<f64>) -> PyResult<Vec<f64>> {
    if parameters.len() != data.len() {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "parameters and data must have the same length ({} vs {})",
            parameters.len(),
            data.len()
        )));
    }
    #[allow(deprecated)]
    {
        Ok(optimize_load(&parameters, &data))
    }
}

/// Multi-day plan: goal is ``"recovery"`` | ``"maintenance"`` | ``"overload"``.
/// Returns ``(daily_tss, form_start, form_end, success, messages)``.
#[pyfunction(name = "optimize_load_plan")]
#[pyo3(signature = (daily_loads, goal, horizon_days=3))]
pub fn py_optimize_load_plan(
    daily_loads: Vec<f64>,
    goal: &str,
    horizon_days: usize,
) -> PyResult<(Vec<f64>, f64, f64, bool, Vec<String>)> {
    let g = match goal.to_ascii_lowercase().as_str() {
        "recovery" | "recover" => LoadGoal::Recovery,
        "maintenance" | "maintain" => LoadGoal::Maintenance,
        "overload" | "load" => LoadGoal::Overload,
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "unknown goal '{other}' (use recovery|maintenance|overload)"
            )));
        }
    };
    let params = PulseResponseParams::pmc_defaults();
    let thr = OptimizationThresholds {
        horizon_days,
        ..Default::default()
    };
    match optimize_load_plan(&daily_loads, &params, g, &thr) {
        Ok(p) => Ok((p.daily_tss, p.form_start, p.form_end, p.success, p.messages)),
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
    }
}

/// PMC pulse-response series: ``(fitness/CTL, fatigue/ATL, form/TSB, performance)``.
#[pyfunction(name = "simulate_pulse_response")]
pub fn py_simulate_pulse_response(
    daily_loads: Vec<f64>,
) -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
    let params = PulseResponseParams::pmc_defaults();
    match simulate_pulse_response(&daily_loads, &params, None) {
        Ok(s) => Ok((s.fitness, s.fatigue, s.form, s.performance)),
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
    }
}

// ==========================================================
// Physiological load
// ==========================================================

#[pyfunction(name = "calculate_physiological_load")]
pub fn py_calculate_physiological_load(hr_data: Vec<f64>) -> PyResult<f64> {
    Ok(calculate_physiological_load(&hr_data))
}

// ==========================================================
// ACWR / Risk (new high-value surface for UNCG + general use)
// ==========================================================

#[pyfunction(name = "compute_acute_chronic")]
pub fn py_compute_acute_chronic(
    daily_loads: Vec<f64>,
    acute_window: usize,
    chronic_window: usize,
) -> PyResult<(f64, f64, f64, String)> {
    match compute_acute_chronic(&daily_loads, acute_window, chronic_window) {
        Ok(s) => Ok((
            s.acute_load,
            s.chronic_load,
            s.acwr,
            s.risk_level.as_str().to_string(),
        )),
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
    }
}

#[pyfunction(name = "compute_ewma_acute_chronic")]
pub fn py_compute_ewma_acute_chronic(
    daily_loads: Vec<f64>,
    acute_span: usize,
    chronic_span: usize,
) -> PyResult<(f64, f64, f64, String)> {
    match compute_ewma_acute_chronic(&daily_loads, acute_span, chronic_span) {
        Ok(s) => Ok((
            s.acute_load,
            s.chronic_load,
            s.acwr,
            s.risk_level.as_str().to_string(),
        )),
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
    }
}

#[pyfunction(name = "classify_acwr")]
pub fn py_classify_acwr(acwr: f64) -> String {
    classify_acwr(acwr).as_str().to_string()
}

#[pyfunction(name = "compute_monotony")]
pub fn py_compute_monotony(daily_loads: Vec<f64>) -> PyResult<f64> {
    compute_monotony(&daily_loads)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

#[pyfunction(name = "compute_strain")]
pub fn py_compute_strain(daily_loads: Vec<f64>) -> PyResult<f64> {
    compute_strain(&daily_loads).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

// ==========================================================
// Ride / Power metrics (critical for .fit / SRM / LoadSym workout analysis)
// ==========================================================

#[pyclass(name = "RideMetrics")]
#[derive(Clone)]
pub struct PyRideMetrics {
    #[pyo3(get)]
    pub duration_s: f64,
    #[pyo3(get)]
    pub total_work_kj: f64,
    #[pyo3(get)]
    pub avg_power: f64,
    #[pyo3(get)]
    pub max_power: f64,
    #[pyo3(get)]
    pub np: f64,
    #[pyo3(get)]
    pub if_: f64,
    #[pyo3(get)]
    pub tss: f64,
}

#[pymethods]
impl PyRideMetrics {
    fn __repr__(&self) -> String {
        format!(
            "RideMetrics(duration={:.1}s, NP={:.0}W, TSS={:.1})",
            self.duration_s, self.np, self.tss
        )
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("duration_s", self.duration_s)?;
        dict.set_item("total_work_kj", self.total_work_kj)?;
        dict.set_item("avg_power", self.avg_power)?;
        dict.set_item("max_power", self.max_power)?;
        dict.set_item("np", self.np)?;
        dict.set_item("if", self.if_)?;
        dict.set_item("tss", self.tss)?;
        Ok(dict.into())
    }
}

impl From<RideMetrics> for PyRideMetrics {
    fn from(m: RideMetrics) -> Self {
        Self {
            duration_s: m.duration_s,
            total_work_kj: m.total_work_kj,
            avg_power: m.avg_power,
            max_power: m.max_power,
            np: m.np,
            if_: m.if_,
            tss: m.tss,
        }
    }
}

#[pyfunction(name = "compute_ride_metrics")]
pub fn py_compute_ride_metrics(times_s: Vec<f64>, power: Vec<f64>, ftp_w: f64) -> PyRideMetrics {
    let m = compute_ride_metrics(&times_s, &power, ftp_w);
    PyRideMetrics::from(m)
}

#[pyfunction(name = "highest_rolling")]
pub fn py_highest_rolling(series: Vec<f64>, window: usize) -> f64 {
    highest_rolling(&series, window)
}

// ==========================================================
// PYTHON REGISTER
// ==========================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_calculate_mechanical_load, m)?)?;
    m.add_function(wrap_pyfunction!(py_optimize_load, m)?)?;
    m.add_function(wrap_pyfunction!(py_optimize_load_plan, m)?)?;
    m.add_function(wrap_pyfunction!(py_simulate_pulse_response, m)?)?;
    m.add_function(wrap_pyfunction!(py_calculate_physiological_load, m)?)?;

    m.add_function(wrap_pyfunction!(py_compute_acute_chronic, m)?)?;
    m.add_function(wrap_pyfunction!(py_compute_ewma_acute_chronic, m)?)?;
    m.add_function(wrap_pyfunction!(py_classify_acwr, m)?)?;
    m.add_function(wrap_pyfunction!(py_compute_monotony, m)?)?;
    m.add_function(wrap_pyfunction!(py_compute_strain, m)?)?;

    m.add_function(wrap_pyfunction!(py_compute_ride_metrics, m)?)?;
    m.add_function(wrap_pyfunction!(py_highest_rolling, m)?)?;
    m.add_class::<PyRideMetrics>()?;

    Ok(())
}
