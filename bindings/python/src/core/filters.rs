// Copyright (c) 2026 SymWorx

use ndarray::{
    Array1,
    Array2,
};
use pyo3::{
    prelude::*,
    wrap_pyfunction,
};
use symworx_core::signal::filters::{
    adaptive::basic::{
        adaptive_mean_filter,
        adaptive_median_filter,
    },
    linear::{
        bandpass::BandpassFilter,
        chebyshev::ChebyshevFilter,
    },
    nonlinear::{
        KalmanFilter,
        KalmanFilter1D,
    },
};

// ==========================================================
// Adaptive filters
// ==========================================================

#[pyfunction(name = "adaptive_mean_filter")]
pub fn py_adaptive_mean_filter(data: Vec<f64>, k: f64) -> PyResult<Vec<f64>> {
    let out = adaptive_mean_filter(&data, k);
    Ok(out)
}

#[pyfunction(name = "adaptive_median_filter")]
pub fn py_adaptive_median_filter(data: Vec<f64>, k: f64) -> PyResult<Vec<f64>> {
    let out = adaptive_median_filter(&data, k);
    Ok(out)
}

// ==========================================================
// Bandpass filter
// ==========================================================

#[pyclass(name = "BandpassFilter")]
pub struct PyBandpassFilter {
    inner: BandpassFilter,
}

#[pymethods]
impl PyBandpassFilter {
    #[new]
    fn new(fs: f64, f_low: f64, f_high: f64, q: f64) -> Self {
        Self {
            inner: BandpassFilter::new(fs, f_low, f_high, q),
        }
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn process(&mut self, data: Vec<f64>) -> Vec<f64> {
        self.inner.process(&data)
    }
}

// ==========================================================
// Chebyshev filter
// ==========================================================

#[pyclass(name = "ChebyshevFilter")]
pub struct PyChebyshevFilter {
    inner: ChebyshevFilter,
}

#[pymethods]
impl PyChebyshevFilter {
    #[new]
    fn new(fs: f64, f_low: f64, f_high: f64, ripple_db: f64) -> Self {
        Self {
            inner: ChebyshevFilter::new(fs, f_low, f_high, ripple_db),
        }
    }

    fn reset(&mut self) {
        self.inner.reset()
    }

    fn process_sample(&mut self, x: f64) -> f64 {
        self.inner.process_sample(x)
    }

    fn process(&mut self, data: Vec<f64>) -> Vec<f64> {
        self.inner.process(&data)
    }
}

// ==========================================================
// Nonlinear Filters
// ==========================================================

// Legacy/simple 1D constant-velocity Kalman filter
#[pyclass(name = "KalmanFilter1D")]
pub struct PyKalmanFilter1D {
    inner: KalmanFilter1D,
}

#[pymethods]
impl PyKalmanFilter1D {
    #[new]
    fn new(dt: f64, process_var: f64, meas_var: f64) -> Self {
        Self {
            inner: KalmanFilter1D::new(dt, process_var, meas_var),
        }
    }

    fn predict(&mut self) {
        self.inner.predict()
    }

    fn update(&mut self, z: f64) {
        self.inner.update(z)
    }

    fn state(&self) -> (f64, f64) {
        self.inner.state()
    }
}

// Primary general state-space Kalman filter (multivariate, control inputs, RTS smoothing)
#[pyclass(name = "KalmanFilter")]
pub struct PyKalmanFilter {
    inner: KalmanFilter,
}

// Helper to convert Vec<Vec<f64>> -> Array2<f64>
fn vec2_to_array2(v: Vec<Vec<f64>>) -> PyResult<Array2<f64>> {
    if v.is_empty() {
        return Ok(Array2::zeros((0, 0)));
    }
    let nrows = v.len();
    let ncols = v[0].len();
    let mut data = Vec::with_capacity(nrows * ncols);
    for row in &v {
        if row.len() != ncols {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "All rows in 2D array must have the same length",
            ));
        }
        data.extend_from_slice(row);
    }
    Ok(Array2::from_shape_vec((nrows, ncols), data)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?)
}

// Helper to convert Vec<f64> -> Array1<f64>
fn vec_to_array1(v: Vec<f64>) -> Array1<f64> {
    Array1::from_vec(v)
}

#[pymethods]
impl PyKalmanFilter {
    /// Create a general KalmanFilter from state-space matrices.
    ///
    /// f: state transition (n x n)
    /// h: measurement matrix (m x n)
    /// q: process noise cov (n x n)
    /// r: measurement noise cov (m x m)
    /// x0: initial state (n,)
    /// p0: initial covariance (n x n)
    #[new]
    fn new(
        f: Vec<Vec<f64>>,
        h: Vec<Vec<f64>>,
        q: Vec<Vec<f64>>,
        r: Vec<Vec<f64>>,
        x0: Vec<f64>,
        p0: Vec<Vec<f64>>,
    ) -> PyResult<Self> {
        let f = vec2_to_array2(f)?;
        let h = vec2_to_array2(h)?;
        let q = vec2_to_array2(q)?;
        let r = vec2_to_array2(r)?;
        let x0 = vec_to_array1(x0);
        let p0 = vec2_to_array2(p0)?;

        Ok(Self {
            inner: KalmanFilter::new(f, h, q, r, x0, p0),
        })
    }

    /// Prediction step. control is optional (length must match control dimension if provided).
    #[pyo3(signature = (control=None))]
    fn predict(&mut self, control: Option<Vec<f64>>) {
        let u = control.map(vec_to_array1);
        self.inner.predict(u.as_ref());
    }

    /// Measurement update with observation vector z.
    fn update(&mut self, z: Vec<f64>) {
        let z = vec_to_array1(z);
        self.inner.update(&z);
    }

    /// Current state estimate as list.
    fn state(&self) -> Vec<f64> {
        self.inner.state().to_vec()
    }

    /// Run forward filter over a sequence of observations.
    /// Returns list of filtered state vectors (one per time step).
    /// controls: optional list of control vectors (same length as zs).
    #[pyo3(signature = (zs, controls=None))]
    fn filter(
        &mut self,
        zs: Vec<Vec<f64>>,
        controls: Option<Vec<Vec<f64>>>,
    ) -> PyResult<Vec<Vec<f64>>> {
        let zs: Vec<Array1<f64>> = zs.into_iter().map(vec_to_array1).collect();
        let controls: Option<Vec<Array1<f64>>> =
            controls.map(|cs| cs.into_iter().map(vec_to_array1).collect());

        let run = self.inner.run_forward(&zs, controls.as_deref());
        Ok(run
            .filtered_states
            .into_iter()
            .map(|s| s.to_vec())
            .collect())
    }
}

// ==========================================================
// Time Frequency
// ==========================================================
// Placeholder

// ==========================================================
// PYTHON REGISTER
// ==========================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_adaptive_mean_filter, m)?)?;
    m.add_function(wrap_pyfunction!(py_adaptive_median_filter, m)?)?;

    m.add_class::<PyBandpassFilter>()?;
    m.add_class::<PyChebyshevFilter>()?;
    m.add_class::<PyKalmanFilter1D>()?;
    m.add_class::<PyKalmanFilter>()?;

    Ok(())
}
