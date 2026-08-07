// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use pyo3::{
    prelude::*,
    wrap_pyfunction,
};
use symworx_core::signal::processing::{
    FillStrategy,
    OutlierCriterion,
    interpolation::{
        interp_cubic,
        interp_linear,
        interp_spline,
        interp1,
    },
    normalization::{
        normalize,
        zscore,
    },
    resample::{
        Resample,
        ResampleMethod,
    },
    resample_rr_to_tachogram,
    robust_interpolate,
};

// ==========================================================
// Normalization
// ==========================================================

#[pyfunction(name = "normalize")]
pub fn py_normalize(data: Vec<f64>) -> Vec<f64> {
    normalize(&data)
}

#[pyfunction(name = "zscore")]
pub fn py_zscore(data: Vec<f64>) -> Vec<f64> {
    zscore(&data)
}

// ==========================================================
// Interpolation
// ==========================================================

#[pyfunction(name = "interp_linear")]
pub fn py_interp_linear(x: Vec<f64>, y: Vec<f64>, x_new: Vec<f64>) -> PyResult<Vec<f64>> {
    Ok(interp_linear(&x, &y, &x_new))
}

#[pyfunction(name = "interp1")]
pub fn py_interp1(x: Vec<f64>, y: Vec<f64>, x_new: Vec<f64>) -> PyResult<Vec<f64>> {
    Ok(interp1(&x, &y, &x_new))
}

#[pyfunction(name = "interp_cubic")]
pub fn py_interp_cubic(x: Vec<f64>, y: Vec<f64>, x_new: Vec<f64>) -> PyResult<Vec<f64>> {
    eprintln!("Warning: cubic interpolation not implemented");
    Ok(interp_cubic(&x, &y, &x_new))
}

#[pyfunction(name = "interp_spline")]
pub fn py_interp_spline(x: Vec<f64>, y: Vec<f64>, x_new: Vec<f64>) -> PyResult<Vec<f64>> {
    eprintln!("Warning: spline interpolation not implemented");
    Ok(interp_spline(&x, &y, &x_new))
}

// ==========================================================
// Resmpling
// ==========================================================

#[pyclass(name = "ResampleMethod")]
#[derive(Clone)]
pub struct PyResampleMethod {
    pub inner: ResampleMethod,
}

#[pymethods]
impl PyResampleMethod {
    #[classattr]
    pub const LINEAR: Self = Self {
        inner: ResampleMethod::Linear,
    };

    #[classattr]
    pub const CUBIC: Self = Self {
        inner: ResampleMethod::Cubic,
    };

    #[classattr]
    pub const SPLINE: Self = Self {
        inner: ResampleMethod::Spline,
    };
}

#[pyclass(name = "Resample")]
pub struct PyResample {
    inner: Resample<'static>,
    data: Vec<f64>,
}

#[pymethods]
impl PyResample {
    #[new]
    #[allow(unsafe_code)]
    pub fn new(y: Vec<f64>) -> Self {
        let data = y;

        // SAFETY: We keep the original `data: Vec<f64>` alive in the same
        // struct for the entire lifetime of `PyResample`. The transmute to
        // `'static` is only to satisfy the `Resample<'static>` requirement
        // (an implementation detail of the resampling helper). The backing
        // data is never mutated while the reference is held by the inner
        // Resample, and the slice is derived directly from `data`.
        let slice: &'static [f64] = unsafe { std::mem::transmute::<&[f64], &'static [f64]>(&data) };

        Self {
            inner: Resample::new(slice),
            data,
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_len(&mut self, n: usize) {
        self.inner.to_len(n);
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn scale(&mut self, factor: f64) {
        self.inner.scale(factor);
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_rate(&mut self, old_fs: f64, new_fs: f64) {
        self.inner.to_rate(old_fs, new_fs);
    }

    pub fn method(&self, method: &PyResampleMethod) -> Vec<f64> {
        self.inner.method(method.inner)
    }
}

// ==========================================================
// Robust Interpolation / Outlier Correction ("dynamics interpolation")
// ==========================================================

#[pyclass(name = "OutlierCriterion")]
#[derive(Clone)]
pub struct PyOutlierCriterion {
    pub inner: OutlierCriterion,
}

#[pymethods]
impl PyOutlierCriterion {
    #[staticmethod]
    pub fn local_mad(half_window: usize, k: f64) -> Self {
        Self {
            inner: OutlierCriterion::LocalMAD { half_window, k },
        }
    }

    #[staticmethod]
    pub fn percent_change(threshold: f64) -> Self {
        Self {
            inner: OutlierCriterion::PercentChange(threshold),
        }
    }

    #[staticmethod]
    pub fn absolute(threshold: f64) -> Self {
        Self {
            inner: OutlierCriterion::Absolute(threshold),
        }
    }
}

#[pyclass(name = "FillStrategy")]
#[derive(Clone)]
pub struct PyFillStrategy {
    pub inner: FillStrategy,
}

#[pymethods]
impl PyFillStrategy {
    #[staticmethod]
    pub fn local_median(half_window: usize) -> Self {
        Self {
            inner: FillStrategy::LocalMedian { half_window },
        }
    }

    #[staticmethod]
    pub fn local_mean(half_window: usize) -> Self {
        Self {
            inner: FillStrategy::LocalMean { half_window },
        }
    }

    #[classattr]
    pub const LINEAR_INTERP: Self = Self {
        inner: FillStrategy::LinearInterp,
    };
}

#[pyfunction(name = "robust_interpolate")]
pub fn py_robust_interpolate(data: Vec<f64>, criterion: &PyOutlierCriterion, strategy: &PyFillStrategy) -> Vec<f64> {
    robust_interpolate(&data, criterion.inner, strategy.inner)
}

#[pyfunction(name = "resample_rr_to_tachogram")]
pub fn py_resample_rr_to_tachogram(event_times: Vec<f64>, interval_values: Vec<f64>, target_fs: f64) -> Vec<f64> {
    resample_rr_to_tachogram(&event_times, &interval_values, target_fs)
}

// ==========================================================
// PYTHON REGISTER
// ==========================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_interp_linear, m)?)?;
    m.add_function(wrap_pyfunction!(py_interp1, m)?)?;
    m.add_function(wrap_pyfunction!(py_interp_cubic, m)?)?;
    m.add_function(wrap_pyfunction!(py_interp_spline, m)?)?;
    m.add_function(wrap_pyfunction!(py_normalize, m)?)?;
    m.add_function(wrap_pyfunction!(py_zscore, m)?)?;

    m.add_function(wrap_pyfunction!(py_robust_interpolate, m)?)?;
    m.add_function(wrap_pyfunction!(py_resample_rr_to_tachogram, m)?)?;

    m.add_class::<PyResampleMethod>()?;
    m.add_class::<PyResample>()?;

    m.add_class::<PyOutlierCriterion>()?;
    m.add_class::<PyFillStrategy>()?;

    Ok(())
}
