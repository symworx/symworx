// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! StatsSym types (views, lab tasks, pipeline metrics).

/// StatsSym sub-tabs (parallel to BioSym Import · Explore · Dynamics).
/// Order is left→right for Ctrl+arrows and the footer tab strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatsView {
    /// File discovery + path load (entry point, like BioSym Import).
    #[default]
    Import,
    /// Analysis workspace (like BioSym Explore).
    Lab,
    /// Synthetic teaching presets (also via Ctrl+G).
    Generate,
}

impl StatsView {
    pub const ALL: [StatsView; 3] = [StatsView::Import, StatsView::Lab, StatsView::Generate];

    pub fn title(self) -> &'static str {
        match self {
            Self::Import => "Import",
            Self::Lab => "Lab",
            Self::Generate => "Generate",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Import => 0,
            Self::Lab => 1,
            Self::Generate => 2,
        }
    }

    pub fn from_index(i: usize) -> Self {
        Self::ALL[i.min(Self::ALL.len() - 1)]
    }

    pub fn prev(self) -> Self {
        let i = self.index();
        if i == 0 { self } else { Self::from_index(i - 1) }
    }

    pub fn next(self) -> Self {
        Self::from_index(self.index().saturating_add(1).min(Self::ALL.len() - 1))
    }
}

/// Guided Lab tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatsLabTask {
    #[default]
    Describe,
    Correlate,
    /// OLS regression + fit / residual plots (single X).
    Regress,
    /// Univariate polynomial degree search (Fit · Poly).
    Poly,
    /// Logistic classification (binary or multiclass OVR).
    Classify,
    /// Train/test + k-fold evaluation (OLS multi-X for now).
    Pipeline,
}

impl StatsLabTask {
    pub const ALL: [StatsLabTask; 6] = [
        StatsLabTask::Describe,
        StatsLabTask::Correlate,
        StatsLabTask::Regress,
        StatsLabTask::Poly,
        StatsLabTask::Classify,
        StatsLabTask::Pipeline,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Describe => "Describe",
            Self::Correlate => "Correlate",
            Self::Regress => "Fit OLS",
            Self::Poly => "Fit Poly",
            Self::Classify => "Classify",
            Self::Pipeline => "Pipeline",
        }
    }
}

/// Bottom residual panel mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResidualPanelMode {
    #[default]
    BlandAltman,
    Histogram,
}

/// Kind of metrics shown in the Pipeline splits table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitMetricKind {
    #[default]
    Regression,
    Classification,
}

/// Pipeline evaluation model (what Pipeline fits on each split).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PipelineModel {
    #[default]
    Ols,
    /// Logistic: binary if 2 classes, OVR if 3+.
    Logistic,
}

impl PipelineModel {
    pub const ALL: [PipelineModel; 2] = [PipelineModel::Ols, PipelineModel::Logistic];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ols => "OLS",
            Self::Logistic => "Logistic",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Ols => Self::Logistic,
            Self::Logistic => Self::Ols,
        }
    }
}

/// One row in a Lab metrics table (Pipeline splits **or** Poly degrees).
#[derive(Debug, Clone, Default)]
pub struct SplitMetricsRow {
    /// Display label: "Full", "Train", "d=2", …
    pub label: String,
    pub n: usize,
    /// Regression: R². Classification: accuracy.
    pub r2: f64,
    /// Regression: RMSE. Classification: balanced accuracy.
    pub rmse: f64,
    /// Regression: MAE. Classification: macro-F1.
    pub mae: f64,
    pub metric_kind: SplitMetricKind,
    /// Short note (e.g. "hold-out", "CV val", "β₀=…").
    pub note: String,
    /// True for the recommended / best-by-R² row (Poly, etc.).
    pub is_best: bool,
    /// Plot: X series (ŷ for multi-feature, or predictor for simple).
    pub plot_x: Vec<f64>,
    /// Plot: observed y.
    pub plot_y: Vec<f64>,
    /// Optional fit overlay X (identity line endpoints or sorted fit).
    pub fit_line_x: Vec<f64>,
    pub fit_line_y: Vec<f64>,
    pub ba_mean: Vec<f64>,
    pub residuals: Vec<f64>,
    pub scatter_x_label: String,
    pub scatter_y_label: String,
    pub is_pred_vs_obs: bool,
}

/// Result of a Lab run (for metrics text + charts).
#[derive(Debug, Clone, Default)]
pub struct StatsLabResult {
    pub task: StatsLabTask,
    pub summary: String,
    pub interpretation: String,
    /// Model name shown in pipeline header (e.g. "OLS").
    pub model_label: String,
    /// Scatter X (predictor or ŷ for multi-feature). Active plot when no split focus.
    pub scatter_x: Vec<f64>,
    /// Observed y.
    pub scatter_y: Vec<f64>,
    /// Fitted line X (sorted) for overlay; empty if N/A.
    pub fit_line_x: Vec<f64>,
    /// Fitted line Y.
    pub fit_line_y: Vec<f64>,
    /// Mean of (y, ŷ) for Bland–Altman X.
    pub ba_mean: Vec<f64>,
    /// Residuals e = y − ŷ for Bland–Altman Y / histogram.
    pub residuals: Vec<f64>,
    /// Axis title for scatter X.
    pub scatter_x_label: String,
    pub scatter_y_label: String,
    /// True when scatter is ŷ vs y (identity line preferred).
    pub is_pred_vs_obs: bool,
    /// Pipeline splits or Poly degrees; plots follow [`Self::focused_row`].
    pub metrics_rows: Vec<SplitMetricsRow>,
    /// Index into [`Self::metrics_rows`] for highlighted row / active plot.
    pub focused_row: usize,
    /// Index of best / recommended row (e.g. Poly best R²); shown under the table.
    pub best_row: Option<usize>,
    /// Extra blurb under the metrics table (best model coeffs, etc.).
    pub table_footer: String,
    /// Left-panel table title: "Splits" | "Degrees" | …
    pub metrics_table_title: String,
}

impl StatsLabResult {
    /// Series used by the fit / residual panels (focused split when present).
    pub fn active_plot(&self) -> (&[f64], &[f64], &[f64], &[f64], &[f64], &[f64], &str, &str, bool) {
        if let Some(row) = self.metrics_rows.get(self.focused_row) {
            (
                &row.plot_x,
                &row.plot_y,
                &row.fit_line_x,
                &row.fit_line_y,
                &row.ba_mean,
                &row.residuals,
                row.scatter_x_label.as_str(),
                row.scatter_y_label.as_str(),
                row.is_pred_vs_obs,
            )
        } else {
            (
                &self.scatter_x,
                &self.scatter_y,
                &self.fit_line_x,
                &self.fit_line_y,
                &self.ba_mean,
                &self.residuals,
                self.scatter_x_label.as_str(),
                self.scatter_y_label.as_str(),
                self.is_pred_vs_obs,
            )
        }
    }

    pub fn focus_next(&mut self) {
        if self.metrics_rows.is_empty() {
            return;
        }
        self.focused_row = (self.focused_row + 1) % self.metrics_rows.len();
    }

    pub fn focus_prev(&mut self) {
        if self.metrics_rows.is_empty() {
            return;
        }
        let n = self.metrics_rows.len();
        self.focused_row = (self.focused_row + n - 1) % n;
    }
}
