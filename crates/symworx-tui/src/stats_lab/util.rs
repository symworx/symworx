// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Shared helpers for Lab runners.

use ndarray::{
    Array1,
    Array2,
};
use symworx_io::TableData;
use symworx_stats::{
    mae,
    r2,
    residuals,
    rmse,
};

use crate::app::{
    SplitMetricKind,
    SplitMetricsRow,
    StatsLabResult,
    StatsLabTask,
};

pub fn col_name(table: &TableData, i: usize) -> String {
    table.headers.get(i).cloned().unwrap_or_else(|| format!("col{i}"))
}

pub fn empty_result(task: StatsLabTask) -> StatsLabResult {
    StatsLabResult {
        task,
        model_label: String::new(),
        metrics_rows: Vec::new(),
        focused_row: 0,
        best_row: None,
        table_footer: String::new(),
        metrics_table_title: String::new(),
        ..Default::default()
    }
}

pub fn fold_mean_sd(vals: &[f64], name: &str) -> String {
    if vals.is_empty() {
        return "no CV folds".into();
    }
    let mean_f = vals.iter().sum::<f64>() / vals.len() as f64;
    let var = vals.iter().map(|v| (v - mean_f).powi(2)).sum::<f64>() / vals.len() as f64;
    format!("CV {}-fold mean {name}={mean_f:.4}  sd={:.4}", vals.len(), var.sqrt())
}

pub fn seed_active_from_focus(r: &mut StatsLabResult) {
    if let Some(row) = r.metrics_rows.get(r.focused_row) {
        r.scatter_x = row.plot_x.clone();
        r.scatter_y = row.plot_y.clone();
        r.fit_line_x = row.fit_line_x.clone();
        r.fit_line_y = row.fit_line_y.clone();
        r.ba_mean = row.ba_mean.clone();
        r.residuals = row.residuals.clone();
        r.scatter_x_label = row.scatter_x_label.clone();
        r.scatter_y_label = row.scatter_y_label.clone();
        r.is_pred_vs_obs = row.is_pred_vs_obs;
    }
}

pub fn metrics_row_reg(label: &str, note: &str, y_obs: &[f64], y_hat: &[f64], y_name: &str) -> SplitMetricsRow {
    let n = y_obs.len().min(y_hat.len());
    let y_obs = &y_obs[..n];
    let y_hat = &y_hat[..n];
    let res = residuals(y_obs, y_hat);
    let r2v = r2(y_obs, y_hat);
    let rmsev = rmse(y_obs, y_hat);
    let maev = mae(y_obs, y_hat);
    let ba: Vec<f64> = y_obs.iter().zip(y_hat.iter()).map(|(&a, &p)| (a + p) / 2.0).collect();
    let mut lo = y_obs.iter().chain(y_hat.iter()).copied().fold(f64::INFINITY, f64::min);
    let mut hi = y_obs
        .iter()
        .chain(y_hat.iter())
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    if !lo.is_finite() {
        lo = 0.0;
        hi = 1.0;
    }
    SplitMetricsRow {
        label: label.into(),
        n,
        r2: r2v,
        rmse: rmsev,
        mae: maev,
        metric_kind: SplitMetricKind::Regression,
        note: note.into(),
        is_best: false,
        plot_x: y_hat.to_vec(),
        plot_y: y_obs.to_vec(),
        fit_line_x: vec![lo, hi],
        fit_line_y: vec![lo, hi],
        ba_mean: ba,
        residuals: res,
        scatter_x_label: "Predicted ŷ".into(),
        scatter_y_label: format!("Observed {y_name}"),
        is_pred_vs_obs: true,
    }
}

pub fn rows_at(x: &Array2<f64>, idx: &[usize]) -> Array2<f64> {
    let p = x.ncols();
    let mut out = Array2::zeros((idx.len(), p));
    for (r, &i) in idx.iter().enumerate() {
        out.row_mut(r).assign(&x.row(i));
    }
    out
}

pub fn array1_at(y: &Array1<f64>, idx: &[usize]) -> Array1<f64> {
    Array1::from(idx.iter().map(|&i| y[i]).collect::<Vec<_>>())
}
