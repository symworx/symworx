// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Logistic classification Lab task (binary + multiclass OVR).

use ndarray::{
    Array1,
    Array2,
};
use symworx_io::TableData;
use symworx_stats::{
    classification_report,
    logistic_regression,
    logistic_regression_ovr,
    LogisticConfig,
};

use super::util::{
    col_name,
    empty_result,
};
use crate::app::{
    StatsLabResult,
    StatsLabTask,
};

pub fn run_classify(
    table: &TableData,
    x_col: usize,
    y_col: usize,
) -> Result<StatsLabResult, String> {
    let n = table.n_rows();
    let p = table.n_cols();
    if n < 10 {
        return Err("Need at least 10 rows for classification".into());
    }
    if p < 2 {
        return Err("Need features + label column".into());
    }
    let y_col = y_col.min(p - 1);
    let y_name = col_name(table, y_col);
    let y_raw = &table.columns[y_col];

    // Features: all other columns (ML-style); if only one other, use x_col preference.
    let mut feature_idx: Vec<usize> = (0..p).filter(|&i| i != y_col).collect();
    if feature_idx.is_empty() {
        return Err("Need feature columns besides y".into());
    }
    // Prefer listing x_col first for single-feature plots when it is a feature.
    if feature_idx.len() > 1 && feature_idx.contains(&x_col) && x_col != y_col {
        feature_idx.retain(|&i| i != x_col);
        feature_idx.insert(0, x_col);
    }
    let feat_names: Vec<String> = feature_idx.iter().map(|&i| col_name(table, i)).collect();
    let n_feat = feature_idx.len();
    let mut x = Array2::<f64>::zeros((n, n_feat));
    for (j, &ci) in feature_idx.iter().enumerate() {
        for i in 0..n {
            x[[i, j]] = table.columns[ci][i];
        }
    }

    let (labels_usize, class_values) = encode_class_labels(y_raw)?;
    let k = class_values.len();
    let config = LogisticConfig {
        max_iter: 4000,
        learning_rate: 0.25,
        ..Default::default()
    };

    if k == 2 {
        let y_bin = Array1::from(labels_usize.iter().map(|&c| c as f64).collect::<Vec<_>>());
        let model = logistic_regression(&x, &y_bin, &config);
        let proba = model.predict_proba(&x);
        let pred = model.predict(&x, 0.5);
        let y_pred: Vec<usize> = pred.iter().map(|&v| if v >= 0.5 { 1 } else { 0 }).collect();
        let rep = classification_report(&labels_usize, &y_pred, Some(2));
        let cm_s = format_conf_mat(&rep.confusion, &class_values);
        let prf_s = format_per_class_prf(&rep, &class_values);

        let (sx, sy, x_lab, y_lab) = if n_feat == 1 {
            (
                table.columns[feature_idx[0]].clone(),
                proba.to_vec(),
                feat_names[0].clone(),
                "P(class=1)".into(),
            )
        } else if n_feat >= 2 {
            // Feature plane: x0 vs x1 (structure of continuous × group)
            (
                table.columns[feature_idx[0]].clone(),
                table.columns[feature_idx[1]].clone(),
                feat_names[0].clone(),
                feat_names[1].clone(),
            )
        } else {
            (
                (0..n).map(|i| i as f64).collect(),
                proba.to_vec(),
                "index".into(),
                "P(class=1)".into(),
            )
        };

        let mut r = empty_result(StatsLabTask::Classify);
        r.model_label = "Logistic binary".into();
        r.summary = format!(
            "Classify · logistic binary  y={y_name}  X=[{}]\n  \
             classes {:?} → {{0,1}}\n  intercept={:.4}  β={:?}\n  \
             acc={:.4}  bal_acc={:.4}  macro_F1={:.4}\n  {prf_s}\n  {cm_s}\n  \
             loss={:.4}  iters={}  conv={}",
            feat_names.join(", "),
            class_values,
            model.intercept,
            model.coefficients.to_vec(),
            rep.accuracy,
            rep.balanced_accuracy,
            rep.macro_f1,
            model.loss,
            model.iterations,
            model.converged,
        );
        r.interpretation = format!(
            "Acc {:.1}% · bal_acc {:.1}% · macro-F1 {:.3}. \
             Plot: {}  ·  Pipeline+m Logistic for CV Acc table.",
            rep.accuracy * 100.0,
            rep.balanced_accuracy * 100.0,
            rep.macro_f1,
            if n_feat >= 2 {
                "feature plane (x1 vs x2)"
            } else {
                "P(class=1)"
            }
        );
        r.scatter_x = sx;
        r.scatter_y = sy;
        if n_feat == 1 {
            let xmin = r.scatter_x.iter().copied().fold(f64::INFINITY, f64::min);
            let xmax = r
                .scatter_x
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            r.fit_line_x = vec![xmin, xmax];
            r.fit_line_y = vec![0.5, 0.5];
        }
        r.scatter_x_label = x_lab;
        r.scatter_y_label = y_lab;
        r.is_pred_vs_obs = false;
        r.residuals = proba
            .iter()
            .zip(y_bin.iter())
            .map(|(&p, &yi)| p - yi)
            .collect();
        r.ba_mean = proba
            .iter()
            .zip(y_bin.iter())
            .map(|(&p, &yi)| (p + yi) / 2.0)
            .collect();
        Ok(r)
    } else {
        // Multiclass OVR — continuous X × K-group teaching path
        let model = logistic_regression_ovr(&x, &labels_usize, &config);
        let pred = model.predict(&x);
        let rep = classification_report(&labels_usize, &pred, Some(k));
        let cm_s = format_conf_mat(&rep.confusion, &class_values);
        let prf_s = format_per_class_prf(&rep, &class_values);
        let proba = model.predict_proba(&x);
        let max_p: Vec<f64> = (0..n)
            .map(|i| (0..k).map(|c| proba[[i, c]]).fold(0.0_f64, f64::max))
            .collect();
        let pred_f: Vec<f64> = pred.iter().map(|&c| c as f64).collect();
        let true_f: Vec<f64> = labels_usize.iter().map(|&c| c as f64).collect();

        // Prefer feature plane when ≥2 features; else true vs pred labels
        let (sx, sy, x_lab, y_lab, is_pred_vs_obs, fit_x, fit_y) = if n_feat >= 2 {
            (
                table.columns[feature_idx[0]].clone(),
                table.columns[feature_idx[1]].clone(),
                feat_names[0].clone(),
                feat_names[1].clone(),
                false,
                vec![],
                vec![],
            )
        } else {
            let lo = 0.0;
            let hi = (k - 1) as f64;
            (
                true_f.clone(),
                pred_f.clone(),
                format!("true {y_name}"),
                "predicted class".into(),
                true,
                vec![lo, hi],
                vec![lo, hi],
            )
        };

        let mut r = empty_result(StatsLabTask::Classify);
        r.model_label = format!("Logistic OVR k={k}");
        r.summary = format!(
            "Classify · logistic OVR (continuous X × {k} groups)  y={y_name}\n  \
             X=[{}]  classes {:?}\n  \
             acc={:.4}  bal_acc={:.4}  macro_P={:.4}  macro_R={:.4}  macro_F1={:.4}\n  \
             {prf_s}\n  {cm_s}",
            feat_names.join(", "),
            class_values,
            rep.accuracy,
            rep.balanced_accuracy,
            rep.macro_precision,
            rep.macro_recall,
            rep.macro_f1,
        );
        r.interpretation = format!(
            "One-vs-rest multiclass. Acc {:.1}% · bal_acc {:.1}% · macro-F1 {:.3}. \
             Diagonal of confusion = correct. \
             Pipeline → m Logistic → Enter for Acc/F1 per split. ThreeClassBlobs demo.",
            rep.accuracy * 100.0,
            rep.balanced_accuracy * 100.0,
            rep.macro_f1,
        );
        r.scatter_x = sx;
        r.scatter_y = sy;
        r.fit_line_x = fit_x;
        r.fit_line_y = fit_y;
        r.scatter_x_label = x_lab;
        r.scatter_y_label = y_lab;
        r.is_pred_vs_obs = is_pred_vs_obs;
        // Error indicator: 0 correct, 1 wrong (as residual height)
        r.residuals = labels_usize
            .iter()
            .zip(pred.iter())
            .map(|(t, p)| if t == p { 0.0 } else { 1.0 })
            .collect();
        r.ba_mean = max_p;
        Ok(r)
    }
}

/// Round continuous labels to nearest integer class levels (stable order).
pub fn encode_class_labels(y: &[f64]) -> Result<(Vec<usize>, Vec<f64>), String> {
    if y.is_empty() {
        return Err("empty labels".into());
    }
    let rounded: Vec<i64> = y
        .iter()
        .map(|&v| if !v.is_finite() { 0 } else { v.round() as i64 })
        .collect();
    let mut uniq = rounded.clone();
    uniq.sort_unstable();
    uniq.dedup();
    if uniq.len() < 2 {
        return Err(format!(
            "Need ≥2 distinct class levels in y (got {} unique after rounding)",
            uniq.len()
        ));
    }
    if uniq.len() > 12 {
        return Err(format!(
            "Too many class levels ({}) — y may be continuous; pick a label column",
            uniq.len()
        ));
    }
    let class_values: Vec<f64> = uniq.iter().map(|&u| u as f64).collect();
    let labels: Vec<usize> = rounded
        .iter()
        .map(|v| uniq.iter().position(|u| u == v).unwrap_or(0))
        .collect();
    Ok((labels, class_values))
}

pub fn format_conf_mat(cm: &ndarray::Array2<usize>, class_values: &[f64]) -> String {
    let k = cm.nrows().min(cm.ncols()).min(class_values.len());
    if k == 0 {
        return "cm: —".into();
    }
    let mut s = String::from("confusion (rows=true, cols=pred):\n");
    s.push_str("       ");
    for c in 0..k {
        s.push_str(&format!("{:>6.0}", class_values[c]));
    }
    s.push('\n');
    for i in 0..k {
        s.push_str(&format!("  {:>4.0}", class_values[i]));
        for j in 0..k {
            s.push_str(&format!("{:>6}", cm[[i, j]]));
        }
        s.push('\n');
    }
    s
}

pub fn format_per_class_prf(
    rep: &symworx_stats::ClassificationReport,
    class_values: &[f64],
) -> String {
    let k = rep
        .n_classes
        .min(class_values.len())
        .min(rep.precision.len())
        .min(rep.recall.len())
        .min(rep.f1.len());
    if k == 0 {
        return "per-class: —".into();
    }
    let mut s = String::from("per-class  P      R      F1\n");
    for i in 0..k {
        s.push_str(&format!(
            "  {:>5.0}  {:.3}  {:.3}  {:.3}\n",
            class_values[i], rep.precision[i], rep.recall[i], rep.f1[i]
        ));
    }
    s
}

pub fn logistic_config() -> LogisticConfig {
    LogisticConfig {
        max_iter: 4000,
        learning_rate: 0.25,
        ..Default::default()
    }
}
