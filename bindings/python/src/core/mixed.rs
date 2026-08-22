// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! PyO3 wrappers for linear mixed models (`symworx-stats::mixed`).

use std::collections::HashMap;

use ndarray::{
    Array1,
    Array2,
};
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    wrap_pyfunction,
};
use symworx_core::stats::{
    EstimationMethod,
    LmerConfig,
    MixedModel as RustMixed,
    RandomInterceptSimSpec,
    RandomTerm,
    generate_random_intercept,
    lmer,
};

fn array2_from_rows(x: Vec<Vec<f64>>) -> PyResult<Array2<f64>> {
    if x.is_empty() {
        return Err(PyValueError::new_err("X must be non-empty"));
    }
    let ncols = x[0].len();
    if x.iter().any(|row| row.len() != ncols) {
        return Err(PyValueError::new_err("All rows of X must have the same length"));
    }
    let nrows = x.len();
    let flat: Vec<f64> = x.into_iter().flatten().collect();
    Array2::from_shape_vec((nrows, ncols), flat).map_err(|e| PyValueError::new_err(format!("Invalid X shape: {e}")))
}

fn parse_method(s: &str) -> PyResult<EstimationMethod> {
    match s.to_ascii_lowercase().as_str() {
        "reml" => Ok(EstimationMethod::Reml),
        "ml" => Ok(EstimationMethod::Ml),
        other => Err(PyValueError::new_err(format!(
            "method must be 'reml' or 'ml', got {other:?}"
        ))),
    }
}

/// Fitted linear mixed model (single grouping factor).
#[pyclass(name = "MixedModel", module = "symworx.core.statistics")]
pub struct PyMixedModel {
    inner: RustMixed,
}

#[pymethods]
impl PyMixedModel {
    #[getter]
    fn intercept(&self) -> f64 {
        self.inner.fixed.intercept
    }

    #[getter]
    fn coefficients(&self) -> Vec<f64> {
        self.inner.fixed.coefficients.to_vec()
    }

    #[getter]
    fn sigma2(&self) -> f64 {
        self.inner.sigma2
    }

    #[getter]
    fn loglik(&self) -> f64 {
        self.inner.loglik
    }

    #[getter]
    fn method(&self) -> &'static str {
        match self.inner.method {
            EstimationMethod::Reml => "reml",
            EstimationMethod::Ml => "ml",
        }
    }

    #[getter]
    fn n(&self) -> usize {
        self.inner.n
    }

    #[getter]
    fn n_fixed(&self) -> usize {
        self.inner.n_fixed
    }

    #[getter]
    fn converged(&self) -> bool {
        self.inner.converged
    }

    #[getter]
    fn iterations(&self) -> usize {
        self.inner.iterations
    }

    /// Number of groups for `name` (default first factor).
    #[pyo3(signature = (name=None))]
    fn n_groups(&self, name: Option<&str>) -> PyResult<usize> {
        let key = self.factor_name(name)?;
        self.inner
            .n_groups
            .get(&key)
            .copied()
            .ok_or_else(|| PyValueError::new_err(format!("unknown factor {key:?}")))
    }

    /// Random-intercept variance `G[0,0]` for `name`.
    #[pyo3(signature = (name=None))]
    fn sigma_u2(&self, name: Option<&str>) -> PyResult<f64> {
        let key = self.factor_name(name)?;
        self.inner
            .sigma_u2(&key)
            .ok_or_else(|| PyValueError::new_err(format!("no RE covariance for {key:?}")))
    }

    /// BLUPs as a list of rows (`n_groups × q`).
    #[pyo3(signature = (name=None))]
    fn ranef(&self, name: Option<&str>) -> PyResult<Vec<Vec<f64>>> {
        let key = self.factor_name(name)?;
        let m = self
            .inner
            .ranef(&key)
            .ok_or_else(|| PyValueError::new_err(format!("no BLUPs for {key:?}")))?;
        Ok(m.outer_iter().map(|row| row.to_vec()).collect())
    }

    /// Random-effect covariance `G` as nested lists (`q × q`).
    #[pyo3(signature = (name=None))]
    fn re_cov(&self, name: Option<&str>) -> PyResult<Vec<Vec<f64>>> {
        let key = self.factor_name(name)?;
        let g = self
            .inner
            .re_covariance(&key)
            .ok_or_else(|| PyValueError::new_err(format!("no RE covariance for {key:?}")))?;
        Ok(g.outer_iter().map(|row| row.to_vec()).collect())
    }

    /// Population-level prediction (`Xβ`, random effects = 0).
    fn predict(&self, x: Vec<Vec<f64>>) -> PyResult<Vec<f64>> {
        let a = array2_from_rows(x)?;
        Ok(self.inner.predict(&a).to_vec())
    }

    /// Subject-specific prediction `Xβ + Zû`.
    ///
    /// For a random intercept, omit `z`. For linear growth, pass `z` as `n × q`.
    #[pyo3(signature = (x, groups, name=None, z=None))]
    fn predict_conditional(
        &self,
        x: Vec<Vec<f64>>,
        groups: Vec<usize>,
        name: Option<&str>,
        z: Option<Vec<Vec<f64>>>,
    ) -> PyResult<Vec<f64>> {
        let key = self.factor_name(name)?;
        let a = array2_from_rows(x)?;
        if groups.len() != a.nrows() {
            return Err(PyValueError::new_err("groups length must match X rows"));
        }
        let mut gmap: HashMap<String, &[usize]> = HashMap::new();
        gmap.insert(key.clone(), groups.as_slice());
        let z_owned = match z {
            Some(rows) => Some(array2_from_rows(rows)?),
            None => None,
        };
        let mut zmap: HashMap<String, &Array2<f64>> = HashMap::new();
        if let Some(ref zo) = z_owned {
            zmap.insert(key, zo);
        }
        self.inner
            .predict_conditional(&a, &gmap, &zmap)
            .map(|v| v.to_vec())
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn summary(&self) -> String {
        self.inner.summary()
    }

    fn __repr__(&self) -> String {
        format!(
            "MixedModel(intercept={:.4}, n={}, n_fixed={}, sigma2={:.4}, converged={})",
            self.inner.fixed.intercept, self.inner.n, self.inner.n_fixed, self.inner.sigma2, self.inner.converged
        )
    }
}

impl PyMixedModel {
    fn factor_name(&self, name: Option<&str>) -> PyResult<String> {
        if let Some(n) = name {
            return Ok(n.to_string());
        }
        self.inner
            .n_groups
            .keys()
            .next()
            .cloned()
            .ok_or_else(|| PyValueError::new_err("model has no random factors"))
    }
}

/// Fit a single-factor LMM (`kind`: ``"intercept"`` or ``"growth"``).
///
/// `x` is `n × p` **without** an intercept column when `fit_intercept` is true.
/// `growth` requires `time` of length `n`.
#[pyfunction(name = "lmer")]
#[pyo3(signature = (
    y,
    x,
    groups,
    name="subject",
    kind="intercept",
    time=None,
    method="reml",
    fit_intercept=true,
    max_iter=400,
    tol=1e-8,
))]
#[allow(clippy::too_many_arguments)]
pub fn py_lmer(
    y: Vec<f64>,
    x: Vec<Vec<f64>>,
    groups: Vec<usize>,
    name: &str,
    kind: &str,
    time: Option<Vec<f64>>,
    method: &str,
    fit_intercept: bool,
    max_iter: usize,
    tol: f64,
) -> PyResult<PyMixedModel> {
    let y_arr = Array1::from(y);
    let x_arr = array2_from_rows(x)?;
    if y_arr.len() != x_arr.nrows() || groups.len() != y_arr.len() {
        return Err(PyValueError::new_err("y, X rows, and groups must have the same length"));
    }
    let g_arr = Array1::from(groups);
    let term = match kind.to_ascii_lowercase().as_str() {
        "intercept" => RandomTerm::random_intercept(name, g_arr),
        "growth" => {
            let t = time.ok_or_else(|| PyValueError::new_err("kind='growth' requires time"))?;
            let t_arr = Array1::from(t);
            RandomTerm::linear_growth(name, g_arr, &t_arr).map_err(|e| PyValueError::new_err(e.to_string()))?
        }
        other => {
            return Err(PyValueError::new_err(format!(
                "kind must be 'intercept' or 'growth', got {other:?}"
            )));
        }
    };
    let cfg = LmerConfig {
        method: parse_method(method)?,
        max_iter,
        tol,
        fit_intercept,
        ..LmerConfig::default()
    };
    let fit = lmer(&y_arr, &x_arr, &[term], &cfg).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(PyMixedModel { inner: fit })
}

/// Simulate balanced random-intercept data. Returns `(y, x, groups)`.
#[pyfunction(name = "simulate_random_intercept")]
#[pyo3(signature = (
    n_groups=40,
    n_per_group=5,
    intercept=2.0,
    coefficients=vec![1.5],
    sigma2=1.0,
    sigma_u2=4.0,
    seed=42,
))]
pub fn py_simulate_random_intercept(
    n_groups: usize,
    n_per_group: usize,
    intercept: f64,
    coefficients: Vec<f64>,
    sigma2: f64,
    sigma_u2: f64,
    seed: u64,
) -> PyResult<(Vec<f64>, Vec<Vec<f64>>, Vec<usize>)> {
    let spec = RandomInterceptSimSpec {
        n_groups,
        n_per_group,
        intercept,
        coefficients: Array1::from(coefficients),
        sigma2,
        sigma_u2,
        seed,
    };
    let data = generate_random_intercept(&spec).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let x: Vec<Vec<f64>> = data.x.outer_iter().map(|row| row.to_vec()).collect();
    Ok((data.y.to_vec(), x, data.groups.to_vec()))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMixedModel>()?;
    m.add_function(wrap_pyfunction!(py_lmer, m)?)?;
    m.add_function(wrap_pyfunction!(py_simulate_random_intercept, m)?)?;
    Ok(())
}
