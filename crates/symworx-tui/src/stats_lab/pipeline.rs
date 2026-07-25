// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! ML pipeline: train/test + k-fold for OLS and logistic.

use ndarray::{
    Array1,
    Array2,
};
use symworx_io::TableData;
use symworx_stats::{
    classification_report,
    logistic_regression,
    logistic_regression_ovr,
    max_train_folds,
    ols,
    train_test_split,
    LogisticConfig,
    SplitConfig,
};

use super::{
    classify::{
        encode_class_labels,
        logistic_config,
    },
    util::{
        array1_at,
        col_name,
        empty_result,
        fold_mean_sd,
        metrics_row_reg,
        rows_at,
        seed_active_from_focus,
    },
};
use crate::app::{
    PipelineModel,
    SplitMetricKind,
    SplitMetricsRow,
    StatsLabResult,
    StatsLabTask,
};

pub(crate) fn run_pipeline(
    table: &TableData,
    y_col: usize,
    pipeline_k: usize,
    model_kind: PipelineModel,
) -> Result<StatsLabResult, String> {
    let n = table.n_rows();
    let p = table.n_cols();
    if n < 20 {
        return Err("Pipeline needs at least 20 rows for a meaningful hold-out".into());
    }
    if p < 2 {
        return Err("Need at least 2 numeric columns (features + y)".into());
    }
    let y_col = y_col.min(p - 1);
    let y_name = col_name(table, y_col);
    let feature_idx: Vec<usize> = (0..p).filter(|&i| i != y_col).collect();
    if feature_idx.is_empty() {
        return Err("Need feature columns besides y".into());
    }
    let feat_names: Vec<String> = feature_idx.iter().map(|&i| col_name(table, i)).collect();
    let n_feat = feature_idx.len();
    let mut x = Array2::<f64>::zeros((n, n_feat));
    for (j, &ci) in feature_idx.iter().enumerate() {
        for i in 0..n {
            x[[i, j]] = table.columns[ci][i];
        }
    }

    let test_ratio = 0.3;
    let max_k = max_train_folds(n, test_ratio);
    let mut fold_note = String::new();
    let n_folds = if max_k >= 2 {
        let k = pipeline_k.clamp(2, max_k);
        if k != pipeline_k {
            fold_note = format!(" (k clamped {pipeline_k}→{k}, max {max_k})");
        }
        Some(k)
    } else {
        fold_note = " (n too small for CV folds — hold-out only)".into();
        None
    };

    let plan = train_test_split(
        n,
        &SplitConfig {
            test_ratio,
            n_train_folds: n_folds,
            shuffle: true,
            seed: 11,
        },
    )
    .map_err(|e| e.to_string())?;

    match model_kind {
        PipelineModel::Ols => run_pipeline_ols(
            &x,
            &table.columns[y_col],
            &y_name,
            &feat_names,
            &plan,
            &fold_note,
        ),
        PipelineModel::Logistic => {
            let (labels, class_values) = encode_class_labels(&table.columns[y_col])?;
            run_pipeline_logistic(
                &x,
                &labels,
                &class_values,
                &y_name,
                &feat_names,
                &plan,
                &fold_note,
            )
        }
    }
}

pub(crate) fn run_pipeline_ols(
    x: &Array2<f64>,
    y_col: &[f64],
    y_name: &str,
    feat_names: &[String],
    plan: &symworx_stats::TrainTestSplit,
    fold_note: &str,
) -> Result<StatsLabResult, String> {
    let y = Array1::from(y_col.to_vec());
    let mut rows: Vec<SplitMetricsRow> = Vec::new();

    {
        let model = ols(x, &y);
        let pred = model.predict(x);
        rows.push(metrics_row_reg(
            "Full",
            "in-sample",
            &y.to_vec(),
            &pred.to_vec(),
            y_name,
        ));
    }

    let x_tr = rows_at(x, &plan.train_idx);
    let y_tr = array1_at(&y, &plan.train_idx);
    let x_te = rows_at(x, &plan.test_idx);
    let y_te = array1_at(&y, &plan.test_idx);
    let m_tr = ols(&x_tr, &y_tr);
    let pred_tr = m_tr.predict(&x_tr);
    let pred_te = m_tr.predict(&x_te);
    rows.push(metrics_row_reg(
        "Train",
        "fit set",
        &y_tr.to_vec(),
        &pred_tr.to_vec(),
        y_name,
    ));
    rows.push(metrics_row_reg(
        "Test",
        "hold-out",
        &y_te.to_vec(),
        &pred_te.to_vec(),
        y_name,
    ));
    let test_row_idx = rows.len() - 1;
    let r2_te = rows[test_row_idx].r2;
    let r2_tr = rows[test_row_idx - 1].r2;

    let mut fold_r2s: Vec<f64> = Vec::new();
    for f in 0..plan.n_folds() {
        let Some(val_idx) = plan.val_idx(f) else {
            continue;
        };
        let Some(fit_idx) = plan.fit_idx(f) else {
            continue;
        };
        let x_fit = rows_at(x, &fit_idx);
        let y_fit = array1_at(&y, &fit_idx);
        let x_val = rows_at(x, val_idx);
        let y_val = array1_at(&y, val_idx);
        let m = ols(&x_fit, &y_fit);
        let pred = m.predict(&x_val);
        let row = metrics_row_reg(
            &format!("Fold {}", f + 1),
            "CV val",
            &y_val.to_vec(),
            &pred.to_vec(),
            y_name,
        );
        fold_r2s.push(row.r2);
        rows.push(row);
    }

    let fold_summary = fold_mean_sd(&fold_r2s, "R²");
    let coef_s: String = m_tr
        .coefficients
        .iter()
        .enumerate()
        .map(|(j, b)| {
            format!(
                "{}={:.3}",
                feat_names.get(j).map(|s| s.as_str()).unwrap_or("?"),
                b
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let mut r = empty_result(StatsLabTask::Pipeline);
    r.model_label = "OLS".into();
    r.summary = format!(
        "Pipeline · OLS  y={y_name}  X=[{}]\n  train n={}  test n={}{fold_note}\n  {coef_s}",
        feat_names.join(", "),
        plan.train_idx.len(),
        plan.test_idx.len(),
    );
    r.interpretation = format!(
        "Hold-out R²={r2_te:.3} (train {r2_tr:.3}). {fold_summary}. \
         m switches model · ↑↓/f focus split · plots follow ★."
    );
    r.metrics_rows = rows;
    r.focused_row = test_row_idx;
    r.metrics_table_title = "Splits".into();
    r.table_footer =
        format!("Focus ★ = hold-out (Test) by default  ·  R²_test={r2_te:.4}  R²_train={r2_tr:.4}");
    seed_active_from_focus(&mut r);
    Ok(r)
}

pub(crate) fn run_pipeline_logistic(
    x: &Array2<f64>,
    labels: &[usize],
    class_values: &[f64],
    y_name: &str,
    feat_names: &[String],
    plan: &symworx_stats::TrainTestSplit,
    fold_note: &str,
) -> Result<StatsLabResult, String> {
    let k = class_values.len();
    let config = logistic_config();
    let mut rows: Vec<SplitMetricsRow> = Vec::new();

    // Full
    rows.push(fit_predict_clf_row(
        "Full",
        "in-sample",
        x,
        labels,
        labels,
        &(0..labels.len()).collect::<Vec<_>>(),
        class_values,
        y_name,
        &config,
    )?);

    // Train / Test
    let y_tr: Vec<usize> = plan.train_idx.iter().map(|&i| labels[i]).collect();
    let y_te: Vec<usize> = plan.test_idx.iter().map(|&i| labels[i]).collect();
    let x_tr = rows_at(x, &plan.train_idx);
    let x_te = rows_at(x, &plan.test_idx);

    // Train metrics: fit on train, score on train
    rows.push(fit_predict_clf_row(
        "Train",
        "fit set",
        &x_tr,
        &y_tr,
        &y_tr,
        &(0..y_tr.len()).collect::<Vec<_>>(),
        class_values,
        y_name,
        &config,
    )?);

    // Test: fit on train, score on test — need special handling
    rows.push(fit_on_train_score_val_clf(
        "Test",
        "hold-out",
        &x_tr,
        &y_tr,
        &x_te,
        &y_te,
        class_values,
        y_name,
        &config,
    )?);
    let test_row_idx = rows.len() - 1;
    let acc_te = rows[test_row_idx].r2;
    let acc_tr = rows[test_row_idx - 1].r2;

    let mut fold_accs: Vec<f64> = Vec::new();
    for f in 0..plan.n_folds() {
        let Some(val_idx) = plan.val_idx(f) else {
            continue;
        };
        let Some(fit_idx) = plan.fit_idx(f) else {
            continue;
        };
        let y_fit: Vec<usize> = fit_idx.iter().map(|&i| labels[i]).collect();
        let y_val: Vec<usize> = val_idx.iter().map(|&i| labels[i]).collect();
        let x_fit = rows_at(x, &fit_idx);
        let x_val = rows_at(x, val_idx);
        let row = fit_on_train_score_val_clf(
            &format!("Fold {}", f + 1),
            "CV val",
            &x_fit,
            &y_fit,
            &x_val,
            &y_val,
            class_values,
            y_name,
            &config,
        )?;
        fold_accs.push(row.r2);
        rows.push(row);
    }

    let fold_summary = fold_mean_sd(&fold_accs, "Acc");
    let mode = if k == 2 { "binary" } else { "OVR multiclass" };

    let mut r = empty_result(StatsLabTask::Pipeline);
    r.model_label = if k == 2 {
        "Logistic binary".into()
    } else {
        format!("Logistic OVR k={k}")
    };
    r.summary = format!(
        "Pipeline · Logistic {mode}  y={y_name}  X=[{}]\n  \
         classes {:?}  train n={}  test n={}{fold_note}",
        feat_names.join(", "),
        class_values,
        plan.train_idx.len(),
        plan.test_idx.len(),
    );
    r.interpretation = format!(
        "Hold-out Acc={acc_te:.3} (train {acc_tr:.3}). {fold_summary}. \
         Table shows Acc / bal_acc / macro-F1. m → OLS · ↑↓/f focus ★."
    );
    r.metrics_rows = rows;
    r.focused_row = test_row_idx;
    r.metrics_table_title = "Splits".into();
    r.table_footer = format!(
        "Focus ★ = hold-out (Test) by default  ·  Acc_test={acc_te:.4}  Acc_train={acc_tr:.4}"
    );
    seed_active_from_focus(&mut r);
    Ok(r)
}

/// Fit classifier on full x_fit/y_fit, score the same indices (for Full/Train in-sample).
pub(crate) fn fit_predict_clf_row(
    label: &str,
    note: &str,
    x: &Array2<f64>,
    y_fit: &[usize],
    y_score: &[usize],
    _idx: &[usize],
    class_values: &[f64],
    y_name: &str,
    config: &LogisticConfig,
) -> Result<SplitMetricsRow, String> {
    fit_on_train_score_val_clf(
        label,
        note,
        x,
        y_fit,
        x,
        y_score,
        class_values,
        y_name,
        config,
    )
}

pub(crate) fn fit_on_train_score_val_clf(
    label: &str,
    note: &str,
    x_fit: &Array2<f64>,
    y_fit: &[usize],
    x_val: &Array2<f64>,
    y_val: &[usize],
    class_values: &[f64],
    y_name: &str,
    config: &LogisticConfig,
) -> Result<SplitMetricsRow, String> {
    let k = class_values.len();
    if y_fit
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        < 2
    {
        return Err(format!(
            "{label}: need ≥2 classes in fit split (try shuffle/seed or more rows)"
        ));
    }

    let pred: Vec<usize> = if k == 2 {
        let y_bin = Array1::from(y_fit.iter().map(|&c| c as f64).collect::<Vec<_>>());
        let model = logistic_regression(x_fit, &y_bin, config);
        model
            .predict(x_val, 0.5)
            .iter()
            .map(|&v| if v >= 0.5 { 1 } else { 0 })
            .collect()
    } else {
        let model = logistic_regression_ovr(x_fit, y_fit, config);
        model.predict(x_val)
    };

    let rep = classification_report(y_val, &pred, Some(k));
    let true_f: Vec<f64> = y_val.iter().map(|&c| c as f64).collect();
    let pred_f: Vec<f64> = pred.iter().map(|&c| c as f64).collect();
    let lo = 0.0_f64;
    let hi = (k.saturating_sub(1)) as f64;
    let err: Vec<f64> = y_val
        .iter()
        .zip(pred.iter())
        .map(|(t, p)| if t == p { 0.0 } else { 1.0 })
        .collect();
    let conf: Vec<f64> = y_val
        .iter()
        .zip(pred.iter())
        .map(|(t, p)| if t == p { 1.0 } else { 0.0 })
        .collect();

    Ok(SplitMetricsRow {
        label: label.into(),
        n: y_val.len(),
        r2: rep.accuracy,
        rmse: rep.balanced_accuracy,
        mae: rep.macro_f1,
        metric_kind: SplitMetricKind::Classification,
        note: note.into(),
        is_best: false,
        plot_x: true_f,
        plot_y: pred_f,
        fit_line_x: vec![lo, hi],
        fit_line_y: vec![lo, hi],
        ba_mean: conf,
        residuals: err,
        scatter_x_label: format!("true {y_name}"),
        scatter_y_label: "predicted class".into(),
        is_pred_vs_obs: true,
    })
}
