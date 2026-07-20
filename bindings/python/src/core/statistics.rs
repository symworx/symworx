// Copyright (c) 2026 SymWorx

use ndarray::{
    Array1,
    Array2,
};
use numpy::PyArray2;
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::PyDict,
    wrap_pyfunction,
};
use symworx_core::stats::{
    // autocorrelation
    acf,
    // classification metrics
    accuracy as rust_accuracy,
    classification_report as rust_classification_report,
    correlation_matrix_from_vec,
    // distance
    euclidean,
    // naive bayes
    gaussian_nb as rust_gaussian_nb,
    // linreg
    l1,
    l2,
    // logistic
    logistic_regression as rust_logistic_regression,
    logistic_regression_ovr as rust_logistic_ovr,
    // basic
    mad,
    // errors
    mae,
    mean,
    mean_successive_differences,
    median,
    mse,
    pearson_correlation,
    percentile,
    // preprocess
    StandardScaler as RustStandardScaler,
    // split
    SplitConfig,
    // types
    GaussianNbConfig,
    LogisticConfig,
    LogisticModel as RustLogisticModel,
    MulticlassLogisticModel as RustMulticlassLogistic,
    rmse,
    rmssd,
    roc_auc as rust_roc_auc,
    roc_auc_ovr as rust_roc_auc_ovr,
    sd_successive_differences,
    // variability
    successive_differences,
    train_test_split as rust_train_test_split,
    GaussianNb as RustGaussianNb,
};

// ==========================================================
// Helpers
// ==========================================================

fn array2_from_rows(x: Vec<Vec<f64>>) -> PyResult<Array2<f64>> {
    if x.is_empty() {
        return Err(PyValueError::new_err("X must be non-empty"));
    }
    let ncols = x[0].len();
    if ncols == 0 {
        return Err(PyValueError::new_err("X must have at least one feature"));
    }
    if x.iter().any(|row| row.len() != ncols) {
        return Err(PyValueError::new_err("All rows of X must have the same length"));
    }
    let nrows = x.len();
    let flat: Vec<f64> = x.into_iter().flatten().collect();
    Array2::from_shape_vec((nrows, ncols), flat)
        .map_err(|e| PyValueError::new_err(format!("Invalid X shape: {e}")))
}

fn array2_to_vec(a: &Array2<f64>) -> Vec<Vec<f64>> {
    (0..a.nrows())
        .map(|i| (0..a.ncols()).map(|j| a[[i, j]]).collect())
        .collect()
}

// ==========================================================
// Autocorrelation
// ==========================================================

#[pyfunction(name = "acf")]
pub fn py_acf(signal: Vec<f64>, unbiased: bool) -> Vec<f64> {
    acf(&signal, unbiased)
}

// ==========================================================
// Basic statistics
// ==========================================================

#[pyfunction(name = "mean")]
pub fn py_mean(data: Vec<f64>) -> f64 {
    mean(&data)
}

#[pyfunction(name = "median")]
pub fn py_median(data: Vec<f64>) -> f64 {
    median(&data)
}

#[pyfunction(name = "mad")]
pub fn py_mad(data: Vec<f64>) -> f64 {
    let med = median(&data);
    mad(&data, med)
}

#[pyfunction(name = "percentile")]
pub fn py_percentile(data: Vec<f64>, p: Vec<f64>) -> PyResult<Vec<f64>> {
    Ok(percentile(&data, p))
}

// ==========================================================
// Correlation
// ==========================================================

#[pyfunction(name = "pearson_correlation")]
pub fn py_pearson_correlation(data: Vec<Vec<f64>>, col1: usize, col2: usize) -> f64 {
    let arr = correlation_matrix_from_vec(&data);
    pearson_correlation(&arr, col1, col2)
}

#[pyfunction(name = "correlation_matrix")]
pub fn py_correlation_matrix<'py>(
    py: Python<'py>,
    data: Vec<Vec<f64>>,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let arr = correlation_matrix_from_vec(&data);

    // Reconstruct as Vec<Vec<f64>> (cheap for small correlation matrices)
    // and use from_vec2 so the resulting PyArray uses whatever ndarray version
    // the numpy crate was compiled against. This avoids ndarray version
    // mismatches between the workspace (0.15) and numpy (which resolved 0.16).
    let n = arr.nrows();
    let mat: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| arr[[i, j]]).collect())
        .collect();

    Ok(PyArray2::from_vec2(py, &mat)?)
}

#[pyfunction(name = "correlation_matrix_from_vec")]
pub fn py_correlation_matrix_from_vec<'py>(
    py: Python<'py>,
    data: Vec<Vec<f64>>,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let arr = correlation_matrix_from_vec(&data);

    // Reconstruct as Vec<Vec<f64>> (cheap for small correlation matrices)
    // and use from_vec2 so the resulting PyArray uses whatever ndarray version
    // the numpy crate was compiled against. This avoids ndarray version
    // mismatches between the workspace (0.15) and numpy (which resolved 0.16).
    let n = arr.nrows();
    let mat: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| arr[[i, j]]).collect())
        .collect();

    Ok(PyArray2::from_vec2(py, &mat)?)
}

// ==========================================================
// Euclidean distance
// ==========================================================

#[pyfunction(name = "euclidean")]
pub fn py_euclidean(vec1: Vec<f64>, vec2: Vec<f64>) -> f64 {
    euclidean(&vec1, &vec2)
}

// ==========================================================
// Errors
// ==========================================================

#[pyfunction(name = "mae")]
pub fn py_mae(actual: Vec<f64>, predicted: Vec<f64>) -> f64 {
    mae(&actual, &predicted)
}

#[pyfunction(name = "mse")]
pub fn py_mse(actual: Vec<f64>, predicted: Vec<f64>) -> f64 {
    mse(&actual, &predicted)
}

#[pyfunction(name = "rmse")]
pub fn py_rmse(actual: Vec<f64>, predicted: Vec<f64>) -> f64 {
    rmse(&actual, &predicted)
}

// ==========================================================
// Linear Regression
// ==========================================================

#[pyfunction(name = "l1")]
#[pyo3(signature = (x, y, alpha=None, max_iter=None, tol=None))]
pub fn py_l1(
    x: Vec<Vec<f64>>,
    y: Vec<f64>,
    alpha: Option<f64>,
    max_iter: Option<usize>,
    tol: Option<f64>,
) -> PyResult<Vec<f64>> {
    // Convert Python lists to ndarray
    let x_arr = Array2::from_shape_vec((x.len(), x[0].len()), x.into_iter().flatten().collect())
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid X shape: {}", e)))?;

    let y_arr = Array1::from_vec(y);

    // Use defaults if not provided
    let alpha = alpha.unwrap_or(0.1);
    let max_iter = max_iter.unwrap_or(200);
    let tol = tol.unwrap_or(1e-6);

    // Call the Rust Lasso implementation
    let beta = l1(&x_arr, &y_arr, alpha, max_iter, tol);

    Ok(beta.to_vec())
}

#[pyfunction(name = "l2")]
pub fn py_l2(x: Vec<Vec<f64>>, y: Vec<f64>) -> PyResult<Vec<f64>> {
    let x_arr = Array2::from_shape_vec((x.len(), x[0].len()), x.into_iter().flatten().collect())
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid X shape: {}", e)))?;

    let y_arr = Array1::from_vec(y);

    let beta = l2(&x_arr, &y_arr);
    Ok(beta.to_vec())
}

// ==========================================================
// Variabilty
// ==========================================================

#[pyfunction(name = "successive_differences")]
pub fn py_successive_differences(data: Vec<f64>) -> Vec<f64> {
    successive_differences(&data)
}

#[pyfunction(name = "mean_successive_differences")]
pub fn py_mean_successive_differences(data: Vec<f64>) -> f64 {
    mean_successive_differences(&data)
}

#[pyfunction(name = "rmssd")]
pub fn py_rmssd(data: Vec<f64>) -> f64 {
    rmssd(&data)
}

#[pyfunction(name = "sd_successive_differences")]
pub fn py_sd_successive_differences(data: Vec<f64>) -> f64 {
    sd_successive_differences(&data)
}

// ==========================================================
// Train / test split (indices only)
// ==========================================================

/// Index-based train/test partition plan (does not copy data).
#[pyclass(name = "TrainTestSplit")]
#[derive(Clone)]
pub struct PyTrainTestSplit {
    #[pyo3(get)]
    pub n: usize,
    #[pyo3(get)]
    pub train_idx: Vec<usize>,
    #[pyo3(get)]
    pub test_idx: Vec<usize>,
    /// Optional training folds (original-space indices), or empty if none.
    #[pyo3(get)]
    pub folds: Vec<Vec<usize>>,
    #[pyo3(get)]
    pub shuffled: bool,
}

#[pymethods]
impl PyTrainTestSplit {
    fn __repr__(&self) -> String {
        format!(
            "TrainTestSplit(n={}, train={}, test={}, n_folds={})",
            self.n,
            self.train_idx.len(),
            self.test_idx.len(),
            self.folds.len()
        )
    }

    /// Fit-set indices for fold `k` (all train except that fold).
    fn fit_idx(&self, fold: usize) -> PyResult<Vec<usize>> {
        if self.folds.is_empty() {
            return Err(PyValueError::new_err("no folds on this split"));
        }
        if fold >= self.folds.len() {
            return Err(PyValueError::new_err("fold out of range"));
        }
        let mut out = Vec::new();
        for (i, f) in self.folds.iter().enumerate() {
            if i != fold {
                out.extend_from_slice(f);
            }
        }
        Ok(out)
    }

    /// Validation indices for fold `k`.
    fn val_idx(&self, fold: usize) -> PyResult<Vec<usize>> {
        self.folds
            .get(fold)
            .cloned()
            .ok_or_else(|| PyValueError::new_err("fold out of range or no folds"))
    }
}

/// Build an index-only train/test plan for `n` rows.
///
/// Parameters
/// ----------
/// n : int
/// test_ratio : float, default 0.3
/// n_train_folds : int or None
/// shuffle : bool, default True
/// seed : int, default 42
#[pyfunction(name = "train_test_split")]
#[pyo3(signature = (n, test_ratio=0.3, n_train_folds=None, shuffle=true, seed=42))]
pub fn py_train_test_split(
    n: usize,
    test_ratio: f64,
    n_train_folds: Option<usize>,
    shuffle: bool,
    seed: u64,
) -> PyResult<PyTrainTestSplit> {
    let cfg = SplitConfig {
        test_ratio,
        n_train_folds,
        shuffle,
        seed,
    };
    let plan = rust_train_test_split(n, &cfg).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(PyTrainTestSplit {
        n: plan.n,
        train_idx: plan.train_idx,
        test_idx: plan.test_idx,
        folds: plan.folds.unwrap_or_default(),
        shuffled: plan.shuffled,
    })
}

// ==========================================================
// Preprocess
// ==========================================================

#[pyclass(name = "StandardScaler")]
#[derive(Clone)]
pub struct PyStandardScaler {
    inner: RustStandardScaler,
}

#[pymethods]
impl PyStandardScaler {
    #[getter]
    fn mean(&self) -> Vec<f64> {
        self.inner.mean.to_vec()
    }

    #[getter]
    fn scale(&self) -> Vec<f64> {
        self.inner.scale.to_vec()
    }

    fn n_features(&self) -> usize {
        self.inner.n_features()
    }

    /// Transform rows: list of feature vectors → list of scaled vectors.
    fn transform(&self, x: Vec<Vec<f64>>) -> PyResult<Vec<Vec<f64>>> {
        let a = array2_from_rows(x)?;
        if a.ncols() != self.inner.n_features() {
            return Err(PyValueError::new_err("feature dimension mismatch"));
        }
        Ok(array2_to_vec(&self.inner.transform(&a)))
    }

    fn inverse_transform(&self, x: Vec<Vec<f64>>) -> PyResult<Vec<Vec<f64>>> {
        let a = array2_from_rows(x)?;
        Ok(array2_to_vec(&self.inner.inverse_transform(&a)))
    }

    fn __repr__(&self) -> String {
        format!("StandardScaler(n_features={})", self.inner.n_features())
    }
}

/// Fit a StandardScaler on X (list of rows). Returns (scaler, X_scaled).
#[pyfunction(name = "standard_scaler_fit")]
pub fn py_standard_scaler_fit(x: Vec<Vec<f64>>) -> PyResult<(PyStandardScaler, Vec<Vec<f64>>)> {
    let a = array2_from_rows(x)?;
    let (sc, z) = RustStandardScaler::fit_transform(&a);
    Ok((PyStandardScaler { inner: sc }, array2_to_vec(&z)))
}

/// Fit StandardScaler only (no transform).
#[pyfunction(name = "standard_scaler_fit_only")]
pub fn py_standard_scaler_fit_only(x: Vec<Vec<f64>>) -> PyResult<PyStandardScaler> {
    let a = array2_from_rows(x)?;
    Ok(PyStandardScaler {
        inner: RustStandardScaler::fit(&a),
    })
}

// ==========================================================
// Logistic (binary + multiclass OVR)
// ==========================================================

#[pyclass(name = "LogisticModel")]
#[derive(Clone)]
pub struct PyLogisticModel {
    inner: RustLogisticModel,
}

#[pymethods]
impl PyLogisticModel {
    #[getter]
    fn intercept(&self) -> f64 {
        self.inner.intercept
    }

    #[getter]
    fn coefficients(&self) -> Vec<f64> {
        self.inner.coefficients.to_vec()
    }

    #[getter]
    fn loss(&self) -> f64 {
        self.inner.loss
    }

    #[getter]
    fn iterations(&self) -> usize {
        self.inner.iterations
    }

    #[getter]
    fn converged(&self) -> bool {
        self.inner.converged
    }

    /// P(y=1) for each row of X.
    fn predict_proba(&self, x: Vec<Vec<f64>>) -> PyResult<Vec<f64>> {
        let a = array2_from_rows(x)?;
        Ok(self.inner.predict_proba(&a).to_vec())
    }

    /// Hard labels 0/1 with threshold (default 0.5).
    #[pyo3(signature = (x, threshold=0.5))]
    fn predict(&self, x: Vec<Vec<f64>>, threshold: f64) -> PyResult<Vec<f64>> {
        let a = array2_from_rows(x)?;
        Ok(self.inner.predict(&a, threshold).to_vec())
    }

    fn decision_function(&self, x: Vec<Vec<f64>>) -> PyResult<Vec<f64>> {
        let a = array2_from_rows(x)?;
        Ok(self.inner.decision_function(&a).to_vec())
    }

    #[pyo3(signature = (x, y, threshold=0.5))]
    fn accuracy(&self, x: Vec<Vec<f64>>, y: Vec<f64>, threshold: f64) -> PyResult<f64> {
        let a = array2_from_rows(x)?;
        let y_arr = Array1::from(y);
        Ok(self.inner.accuracy(&a, &y_arr, threshold))
    }

    fn __repr__(&self) -> String {
        format!(
            "LogisticModel(intercept={:.4}, n_features={}, converged={})",
            self.inner.intercept,
            self.inner.n_features(),
            self.inner.converged
        )
    }
}

/// Fit binary logistic regression. y must be 0.0 / 1.0.
#[pyfunction(name = "logistic_regression")]
#[pyo3(signature = (x, y, max_iter=1000, learning_rate=0.1, l2=0.0, tol=1e-6, fit_intercept=true))]
pub fn py_logistic_regression(
    x: Vec<Vec<f64>>,
    y: Vec<f64>,
    max_iter: usize,
    learning_rate: f64,
    l2: f64,
    tol: f64,
    fit_intercept: bool,
) -> PyResult<PyLogisticModel> {
    let a = array2_from_rows(x)?;
    let y_arr = Array1::from(y);
    if a.nrows() != y_arr.len() {
        return Err(PyValueError::new_err("X and y length mismatch"));
    }
    let cfg = LogisticConfig {
        max_iter,
        learning_rate,
        l2,
        tol,
        fit_intercept,
        threshold: 0.5,
    };
    // Rust panics on bad labels — catch via catch_unwind would be heavy; document.
    let model = rust_logistic_regression(&a, &y_arr, &cfg);
    Ok(PyLogisticModel { inner: model })
}

#[pyclass(name = "MulticlassLogisticModel")]
#[derive(Clone)]
pub struct PyMulticlassLogisticModel {
    inner: RustMulticlassLogistic,
}

#[pymethods]
impl PyMulticlassLogisticModel {
    #[getter]
    fn classes(&self) -> Vec<usize> {
        self.inner.classes.clone()
    }

    fn n_classes(&self) -> usize {
        self.inner.n_classes()
    }

    fn n_features(&self) -> usize {
        self.inner.n_features()
    }

    fn converged(&self) -> bool {
        self.inner.converged()
    }

    fn mean_loss(&self) -> f64 {
        self.inner.mean_loss()
    }

    /// Class probabilities: list of rows, each length n_classes.
    fn predict_proba(&self, x: Vec<Vec<f64>>) -> PyResult<Vec<Vec<f64>>> {
        let a = array2_from_rows(x)?;
        Ok(array2_to_vec(&self.inner.predict_proba(&a)))
    }

    fn predict(&self, x: Vec<Vec<f64>>) -> PyResult<Vec<usize>> {
        let a = array2_from_rows(x)?;
        Ok(self.inner.predict(&a))
    }

    fn accuracy(&self, x: Vec<Vec<f64>>, y: Vec<usize>) -> PyResult<f64> {
        let a = array2_from_rows(x)?;
        Ok(self.inner.accuracy(&a, &y))
    }

    /// Per-class binary models as LogisticModel objects.
    fn models(&self) -> Vec<PyLogisticModel> {
        self.inner
            .models
            .iter()
            .cloned()
            .map(|m| PyLogisticModel { inner: m })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "MulticlassLogisticModel(classes={:?}, converged={})",
            self.inner.classes,
            self.inner.converged()
        )
    }
}

/// Multiclass logistic via one-vs-rest. y = integer class labels.
#[pyfunction(name = "logistic_regression_ovr")]
#[pyo3(signature = (x, y, max_iter=1000, learning_rate=0.1, l2=0.0, tol=1e-6, fit_intercept=true))]
pub fn py_logistic_regression_ovr(
    x: Vec<Vec<f64>>,
    y: Vec<usize>,
    max_iter: usize,
    learning_rate: f64,
    l2: f64,
    tol: f64,
    fit_intercept: bool,
) -> PyResult<PyMulticlassLogisticModel> {
    let a = array2_from_rows(x)?;
    if a.nrows() != y.len() {
        return Err(PyValueError::new_err("X and y length mismatch"));
    }
    let cfg = LogisticConfig {
        max_iter,
        learning_rate,
        l2,
        tol,
        fit_intercept,
        threshold: 0.5,
    };
    let model = rust_logistic_ovr(&a, &y, &cfg);
    Ok(PyMulticlassLogisticModel { inner: model })
}

// ==========================================================
// Classification metrics + ROC
// ==========================================================

/// Accuracy for integer labels.
#[pyfunction(name = "accuracy")]
pub fn py_accuracy(y_true: Vec<usize>, y_pred: Vec<usize>) -> f64 {
    rust_accuracy(&y_true, &y_pred)
}

/// Classification report as a dict (accuracy, macro_f1, confusion, …).
#[pyfunction(name = "classification_report")]
#[pyo3(signature = (y_true, y_pred, n_classes=None))]
pub fn py_classification_report<'py>(
    py: Python<'py>,
    y_true: Vec<usize>,
    y_pred: Vec<usize>,
    n_classes: Option<usize>,
) -> PyResult<Bound<'py, PyDict>> {
    let rep = rust_classification_report(&y_true, &y_pred, n_classes);
    let d = PyDict::new(py);
    d.set_item("n", rep.n)?;
    d.set_item("n_classes", rep.n_classes)?;
    d.set_item("accuracy", rep.accuracy)?;
    d.set_item("balanced_accuracy", rep.balanced_accuracy)?;
    d.set_item("macro_precision", rep.macro_precision)?;
    d.set_item("macro_recall", rep.macro_recall)?;
    d.set_item("macro_f1", rep.macro_f1)?;
    d.set_item("precision", rep.precision)?;
    d.set_item("recall", rep.recall)?;
    d.set_item("f1", rep.f1)?;
    let cm: Vec<Vec<usize>> = (0..rep.confusion.nrows())
        .map(|i| {
            (0..rep.confusion.ncols())
                .map(|j| rep.confusion[[i, j]])
                .collect()
        })
        .collect();
    d.set_item("confusion", cm)?;
    Ok(d)
}

/// Binary ROC-AUC. y_true in {0,1}; higher scores ⇒ class 1.
#[pyfunction(name = "roc_auc")]
pub fn py_roc_auc(y_true: Vec<usize>, scores: Vec<f64>) -> f64 {
    rust_roc_auc(&y_true, &scores)
}

/// Macro one-vs-rest ROC-AUC. scores = list of rows (n_classes each).
#[pyfunction(name = "roc_auc_ovr")]
#[pyo3(signature = (y_true, scores, classes=None))]
pub fn py_roc_auc_ovr(
    y_true: Vec<usize>,
    scores: Vec<Vec<f64>>,
    classes: Option<Vec<usize>>,
) -> PyResult<f64> {
    let a = array2_from_rows(scores)?;
    let cls_ref = classes.as_deref();
    Ok(rust_roc_auc_ovr(&y_true, &a, cls_ref))
}

// ==========================================================
// Gaussian Naive Bayes
// ==========================================================

#[pyclass(name = "GaussianNb")]
#[derive(Clone)]
pub struct PyGaussianNb {
    inner: RustGaussianNb,
}

#[pymethods]
impl PyGaussianNb {
    #[getter]
    fn classes(&self) -> Vec<usize> {
        self.inner.classes.clone()
    }

    fn predict(&self, x: Vec<Vec<f64>>) -> PyResult<Vec<usize>> {
        let a = array2_from_rows(x)?;
        Ok(self.inner.predict(&a))
    }

    fn predict_proba(&self, x: Vec<Vec<f64>>) -> PyResult<Vec<Vec<f64>>> {
        let a = array2_from_rows(x)?;
        Ok(array2_to_vec(&self.inner.predict_proba(&a)))
    }

    fn __repr__(&self) -> String {
        format!("GaussianNb(classes={:?})", self.inner.classes)
    }
}

/// Fit Gaussian Naive Bayes. y = integer class labels.
#[pyfunction(name = "gaussian_nb")]
#[pyo3(signature = (x, y, var_smoothing=1e-9))]
pub fn py_gaussian_nb(
    x: Vec<Vec<f64>>,
    y: Vec<usize>,
    var_smoothing: f64,
) -> PyResult<PyGaussianNb> {
    let a = array2_from_rows(x)?;
    if a.nrows() != y.len() {
        return Err(PyValueError::new_err("X and y length mismatch"));
    }
    let cfg = GaussianNbConfig { var_smoothing };
    Ok(PyGaussianNb {
        inner: rust_gaussian_nb(&a, &y, &cfg),
    })
}

// ==========================================================
// PYTHON REGISTER
// ==========================================================

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // --- Autocorrelation ----------------------------------
    m.add_function(wrap_pyfunction!(py_acf, m)?)?;

    // --- Basic statistics ---------------------------------
    m.add_function(wrap_pyfunction!(py_mean, m)?)?;
    m.add_function(wrap_pyfunction!(py_median, m)?)?;
    m.add_function(wrap_pyfunction!(py_mad, m)?)?;
    m.add_function(wrap_pyfunction!(py_percentile, m)?)?;

    // --- Correlation --------------------------------------
    let _ = m.add_function(wrap_pyfunction!(py_pearson_correlation, m)?);
    m.add_function(wrap_pyfunction!(py_correlation_matrix, m)?)?;
    let _ = m.add_function(wrap_pyfunction!(py_correlation_matrix_from_vec, m)?);

    // --- Distance -----------------------------------------
    m.add_function(wrap_pyfunction!(py_euclidean, m)?)?;

    // --- Errors -------------------------------------------
    m.add_function(wrap_pyfunction!(py_mae, m)?)?;
    m.add_function(wrap_pyfunction!(py_mse, m)?)?;
    m.add_function(wrap_pyfunction!(py_rmse, m)?)?;

    // --- Linear regression --------------------------------
    m.add_function(wrap_pyfunction!(py_l1, m)?)?;
    m.add_function(wrap_pyfunction!(py_l2, m)?)?;

    // --- Variability --------------------------------------
    m.add_function(wrap_pyfunction!(py_successive_differences, m)?)?;
    m.add_function(wrap_pyfunction!(py_mean_successive_differences, m)?)?;
    m.add_function(wrap_pyfunction!(py_rmssd, m)?)?;
    m.add_function(wrap_pyfunction!(py_sd_successive_differences, m)?)?;

    // --- Split / preprocess / logistic / metrics (0.1 ML surface) ---
    m.add_class::<PyTrainTestSplit>()?;
    m.add_class::<PyStandardScaler>()?;
    m.add_class::<PyLogisticModel>()?;
    m.add_class::<PyMulticlassLogisticModel>()?;
    m.add_class::<PyGaussianNb>()?;
    m.add_function(wrap_pyfunction!(py_train_test_split, m)?)?;
    m.add_function(wrap_pyfunction!(py_standard_scaler_fit, m)?)?;
    m.add_function(wrap_pyfunction!(py_standard_scaler_fit_only, m)?)?;
    m.add_function(wrap_pyfunction!(py_logistic_regression, m)?)?;
    m.add_function(wrap_pyfunction!(py_logistic_regression_ovr, m)?)?;
    m.add_function(wrap_pyfunction!(py_accuracy, m)?)?;
    m.add_function(wrap_pyfunction!(py_classification_report, m)?)?;
    m.add_function(wrap_pyfunction!(py_roc_auc, m)?)?;
    m.add_function(wrap_pyfunction!(py_roc_auc_ovr, m)?)?;
    m.add_function(wrap_pyfunction!(py_gaussian_nb, m)?)?;

    Ok(())
}
