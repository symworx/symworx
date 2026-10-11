// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

use pyo3::{
    prelude::*,
    wrap_pyfunction,
};
use symworx_core::signal::processing::{
    FillStrategy,
    OutlierCriterion,
    burst_indices,
    convolve,
    exponential_kernel,
    gh_burst_deconvolve,
    gh_deconvolve,
    gh_deconvolve_sparse,
    gh_reconvolve,
    interpolation::{
        interp_cubic,
        interp_linear,
        interp_spline,
        interp1,
    },
    nonnegative_deconvolution,
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
    sparse_nonnegative_deconvolution,
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
    Ok(interp_cubic(&x, &y, &x_new))
}

#[pyfunction(name = "interp_spline")]
pub fn py_interp_spline(x: Vec<f64>, y: Vec<f64>, x_new: Vec<f64>) -> PyResult<Vec<f64>> {
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
// Deconvolution (hormone reconstruction)
// ==========================================================

#[pyfunction(name = "exponential_kernel")]
pub fn py_exponential_kernel(dt: f64, tau: f64, len: usize) -> Vec<f64> {
    exponential_kernel(dt, tau, len)
}

#[pyfunction(name = "convolve")]
pub fn py_convolve(signal: Vec<f64>, kernel: Vec<f64>) -> Vec<f64> {
    convolve(&signal, &kernel)
}

#[pyfunction(name = "nonnegative_deconvolution")]
#[pyo3(signature = (observed, kernel, lam=1e-4))]
pub fn py_nonnegative_deconvolution(observed: Vec<f64>, kernel: Vec<f64>, lam: f64) -> Vec<f64> {
    nonnegative_deconvolution(&observed, &kernel, lam)
}

#[pyfunction(name = "gh_deconvolve")]
pub fn py_gh_deconvolve(observed: Vec<f64>, kernel: Vec<f64>) -> Vec<f64> {
    gh_deconvolve(&observed, &kernel)
}

#[pyfunction(name = "gh_reconvolve")]
pub fn py_gh_reconvolve(secretion: Vec<f64>, kernel: Vec<f64>) -> Vec<f64> {
    gh_reconvolve(&secretion, &kernel)
}

#[pyfunction(name = "sparse_nonnegative_deconvolution")]
#[pyo3(signature = (observed, kernel, lam=0.1, max_iter=2000, tol=1e-8))]
pub fn py_sparse_nonnegative_deconvolution(
    observed: Vec<f64>,
    kernel: Vec<f64>,
    lam: f64,
    max_iter: usize,
    tol: f64,
) -> Vec<f64> {
    sparse_nonnegative_deconvolution(&observed, &kernel, lam, max_iter, tol)
}

#[pyfunction(name = "gh_deconvolve_sparse")]
#[pyo3(signature = (observed, kernel, lam=0.1))]
pub fn py_gh_deconvolve_sparse(observed: Vec<f64>, kernel: Vec<f64>, lam: f64) -> Vec<f64> {
    gh_deconvolve_sparse(&observed, &kernel, lam)
}

#[pyfunction(name = "burst_indices")]
#[pyo3(signature = (secretion, floor=0.5))]
pub fn py_burst_indices(secretion: Vec<f64>, floor: f64) -> Vec<usize> {
    burst_indices(&secretion, floor)
}

#[pyfunction(name = "gh_burst_deconvolve")]
#[pyo3(signature = (observed, kernel, lam=0.1, floor=0.5))]
pub fn py_gh_burst_deconvolve(observed: Vec<f64>, kernel: Vec<f64>, lam: f64, floor: f64) -> Vec<f64> {
    gh_burst_deconvolve(&observed, &kernel, lam, floor)
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
    m.add_function(wrap_pyfunction!(py_exponential_kernel, m)?)?;
    m.add_function(wrap_pyfunction!(py_convolve, m)?)?;
    m.add_function(wrap_pyfunction!(py_nonnegative_deconvolution, m)?)?;
    m.add_function(wrap_pyfunction!(py_gh_deconvolve, m)?)?;
    m.add_function(wrap_pyfunction!(py_gh_reconvolve, m)?)?;
    m.add_function(wrap_pyfunction!(py_sparse_nonnegative_deconvolution, m)?)?;
    m.add_function(wrap_pyfunction!(py_gh_deconvolve_sparse, m)?)?;
    m.add_function(wrap_pyfunction!(py_burst_indices, m)?)?;
    m.add_function(wrap_pyfunction!(py_gh_burst_deconvolve, m)?)?;

    m.add_class::<PyResampleMethod>()?;
    m.add_class::<PyResample>()?;

    m.add_class::<PyOutlierCriterion>()?;
    m.add_class::<PyFillStrategy>()?;

    Ok(())
}
