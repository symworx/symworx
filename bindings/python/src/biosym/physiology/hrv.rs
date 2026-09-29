// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

use pyo3::{
    prelude::*,
    wrap_pyfunction,
};
use symworx_biosym::common::{
    HrvMetrics,
    compute_hrv_metrics,
};
use symworx_core::stats::sdnn;

/// Heart rate variability from RR intervals (seconds).
#[pyclass(name = "HrvMetrics")]
#[derive(Clone)]
pub struct PyHrvMetrics {
    /// Root mean square of successive RR differences (seconds).
    /// `None` when fewer than three intervals are supplied.
    #[pyo3(get)]
    pub rmssd_sec: Option<f64>,
    /// Population SDNN (seconds). `None` when fewer than two intervals are supplied.
    #[pyo3(get)]
    pub sdnn_sec: Option<f64>,
}

impl From<HrvMetrics> for PyHrvMetrics {
    fn from(h: HrvMetrics) -> Self {
        Self {
            rmssd_sec: h.rmssd_sec,
            sdnn_sec: h.sdnn_sec,
        }
    }
}

#[pymethods]
impl PyHrvMetrics {
    fn __repr__(&self) -> String {
        format!(
            "HrvMetrics(rmssd_sec={:?}, sdnn_sec={:?})",
            self.rmssd_sec, self.sdnn_sec
        )
    }
}

/// RMSSD and SDNN from inter-beat intervals (seconds).
///
/// `rmssd_sec` is `None` with fewer than three intervals. `sdnn_sec` is `None`
/// with fewer than two.
#[pyfunction(name = "compute_hrv_metrics")]
pub fn py_compute_hrv_metrics(rr_intervals_sec: Vec<f64>) -> PyHrvMetrics {
    compute_hrv_metrics(&rr_intervals_sec).into()
}

/// Population SDNN of an interval series.
///
/// Same units as `rr_intervals_sec`. Fewer than two samples returns NaN.
/// This is the same value as `HrvMetrics.sdnn_sec` when that field is present.
#[pyfunction(name = "sdnn")]
pub fn py_sdnn(rr_intervals_sec: Vec<f64>) -> f64 {
    sdnn(&rr_intervals_sec)
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyHrvMetrics>()?;
    m.add_function(wrap_pyfunction!(py_compute_hrv_metrics, m)?)?;
    m.add_function(wrap_pyfunction!(py_sdnn, m)?)?;
    Ok(())
}
