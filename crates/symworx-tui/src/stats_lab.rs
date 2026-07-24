// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! StatsSym Lab runners: describe, correlate, OLS/poly fit, classify, pipeline.

use ndarray::{
    Array1,
    Array2,
};
use symworx_io::TableData;
use symworx_stats::{
    classification_report,
    fit_polynomial_degrees_with,
    logistic_regression,
    logistic_regression_ovr,
    mae,
    max_train_folds,
    mean,
    median,
    ols,
    r2,
    regression_report,
    residuals,
    rmse,
    std_dev_sample,
    train_test_split,
    LogisticConfig,
    PolynomialSearchConfig,
    SplitConfig,
};

use crate::app::{
    PipelineModel,
    SplitMetricKind,
    SplitMetricsRow,
    StatsLabResult,
    StatsLabTask,
};

/// Lab run options (pipeline k, poly max degree, pipeline model).
#[derive(Debug, Clone, Copy)]
pub struct LabRunOpts {
    pub pipeline_k: usize,
    pub poly_max_degree: usize,
    pub pipeline_model: PipelineModel,
}

impl Default for LabRunOpts {
    fn default() -> Self {
        Self {
            pipeline_k: 5,
            poly_max_degree: 3,
            pipeline_model: PipelineModel::Ols,
        }
    }
}

/// Run the selected Lab task on the current table.
pub fn run_lab(
    table: &TableData,
    task: StatsLabTask,
    x_col: usize,
    y_col: usize,
    opts: LabRunOpts,
) -> Result<StatsLabResult, String> {
    if table.is_empty() {
        return Err("No table loaded".into());
    }
    let p = table.n_cols();
    let x_col = x_col.min(p.saturating_sub(1));
    let y_col = y_col.min(p.saturating_sub(1));

    match task {
        StatsLabTask::Describe => run_describe(table, y_col),
        StatsLabTask::Correlate => run_correlate(table, x_col, y_col),
        StatsLabTask::Regress => run_regress(table, x_col, y_col),
        StatsLabTask::Poly => run_poly(table, x_col, y_col, opts.poly_max_degree),
        StatsLabTask::Classify => run_classify(table, x_col, y_col),
        StatsLabTask::Pipeline => run_pipeline(table, y_col, opts.pipeline_k, opts.pipeline_model),
    }
}

fn col_name(table: &TableData, i: usize) -> String {
    table
        .headers
        .get(i)
        .cloned()
        .unwrap_or_else(|| format!("col{i}"))
}

fn empty_result(task: StatsLabTask) -> StatsLabResult {
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

fn run_describe(table: &TableData, col: usize) -> Result<StatsLabResult, String> {
    let c = &table.columns[col];
    let name = col_name(table, col);
    let m = mean(c);
    let med = median(c);
    let s = std_dev_sample(c);
    let mn = c.iter().copied().fold(f64::INFINITY, f64::min);
    let mx = c.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let summary = format!(
        "Describe · {name}\n  n={}  mean={m:.4}  median={med:.4}\n  sd={s:.4}  min={mn:.4}  max={mx:.4}",
        c.len()
    );
    let interpretation =
        format!("Column “{name}”: centre ≈ {m:.3} (median {med:.3}); spread sd ≈ {s:.3}.");
    let scatter_x: Vec<f64> = (0..c.len()).map(|i| i as f64).collect();
    let mut r = empty_result(StatsLabTask::Describe);
    r.summary = summary;
    r.interpretation = interpretation;
    r.scatter_x = scatter_x;
    r.scatter_y = c.clone();
    r.scatter_x_label = "index".into();
    r.scatter_y_label = name;
    Ok(r)
}

fn run_correlate(table: &TableData, x_col: usize, y_col: usize) -> Result<StatsLabResult, String> {
    if x_col == y_col {
        return Err("Pick two different columns for correlation".into());
    }
    let x = &table.columns[x_col];
    let y = &table.columns[y_col];
    let n = x.len().min(y.len()) as f64;
    if n < 3.0 {
        return Err("Need at least 3 rows".into());
    }
    let mx = mean(x);
    let my = mean(y);
    let mut num = 0.0;
    let mut dx = 0.0;
    let mut dy = 0.0;
    for i in 0..x.len().min(y.len()) {
        let a = x[i] - mx;
        let b = y[i] - my;
        num += a * b;
        dx += a * a;
        dy += b * b;
    }
    let r_coef = if dx > 0.0 && dy > 0.0 {
        num / (dx.sqrt() * dy.sqrt())
    } else {
        0.0
    };
    let xn = col_name(table, x_col);
    let yn = col_name(table, y_col);
    let summary = format!("Correlate · {xn} vs {yn}\n  n={n:.0}  Pearson r={r_coef:.4}");
    let interpretation = if r_coef.abs() > 0.7 {
        format!("Strong linear association (|r|={:.2}).", r_coef.abs())
    } else if r_coef.abs() > 0.4 {
        format!("Moderate linear association (|r|={:.2}).", r_coef.abs())
    } else {
        format!("Weak linear association (|r|={:.2}).", r_coef.abs())
    };
    let mut r = empty_result(StatsLabTask::Correlate);
    r.summary = summary;
    r.interpretation = interpretation;
    r.scatter_x = x[..x.len().min(y.len())].to_vec();
    r.scatter_y = y[..x.len().min(y.len())].to_vec();
    r.scatter_x_label = xn;
    r.scatter_y_label = yn;
    Ok(r)
}

/// Simple OLS on one X column (clear fitted line).
fn run_regress(table: &TableData, x_col: usize, y_col: usize) -> Result<StatsLabResult, String> {
    let n = table.n_rows();
    let p = table.n_cols();
    if n < 5 {
        return Err("Need at least 5 rows for regression".into());
    }
    if p < 2 {
        return Err("Need at least 2 numeric columns (X and y)".into());
    }
    let y_col = y_col.min(p - 1);
    let y_name = col_name(table, y_col);
    let y: Array1<f64> = Array1::from(table.columns[y_col].clone());
    let xc = if x_col == y_col {
        (0..p).find(|&i| i != y_col).unwrap_or(0)
    } else {
        x_col
    };
    let x_name = col_name(table, xc);
    let mut x = Array2::<f64>::zeros((n, 1));
    for i in 0..n {
        x[[i, 0]] = table.columns[xc][i];
    }

    let model = ols(&x, &y);
    let y_hat = model.predict(&x);
    let y_hat_v: Vec<f64> = y_hat.to_vec();
    let y_v: Vec<f64> = y.to_vec();
    let res = residuals(&y_v, &y_hat_v);
    let rep = regression_report(&y_v, &y_hat_v);
    let r2_full = r2(&y_v, &y_hat_v);
    let rmse_full = rmse(&y_v, &y_hat_v);
    let mae_full = mae(&y_v, &y_hat_v);
    let beta = model.coefficients.first().copied().unwrap_or(0.0);

    let xv = table.columns[xc].clone();
    let mut pairs: Vec<(f64, f64, f64)> = xv
        .iter()
        .zip(y_v.iter())
        .zip(y_hat_v.iter())
        .map(|((&a, &b), &yh)| (a, b, yh))
        .collect();
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let fit_x: Vec<f64> = pairs.iter().map(|p| p.0).collect();
    let fit_yh: Vec<f64> = pairs.iter().map(|p| p.2).collect();
    let ba_mean: Vec<f64> = y_v
        .iter()
        .zip(y_hat_v.iter())
        .map(|(&a, &p)| (a + p) / 2.0)
        .collect();

    let mut r = empty_result(StatsLabTask::Regress);
    r.model_label = "OLS".into();
    r.summary = format!(
        "OLS regress · y={y_name}  X={x_name}\n  intercept={:.4}  β={beta:.4}\n  R²={r2_full:.4}  RMSE={rmse_full:.4}  MAE={mae_full:.4}",
        model.intercept,
    );
    r.interpretation = format!(
        "Model explains ~{:.0}% of variance (R²). Mean residual (bias) ≈ {:.4}.",
        r2_full * 100.0,
        rep.bias
    );
    r.scatter_x = xv;
    r.scatter_y = y_v;
    r.fit_line_x = fit_x;
    r.fit_line_y = fit_yh;
    r.ba_mean = ba_mean;
    r.residuals = res;
    r.scatter_x_label = x_name;
    r.scatter_y_label = y_name;
    r.is_pred_vs_obs = false;
    Ok(r)
}

/// Univariate polynomial degree search — left table of degrees, right plots.
fn run_poly(
    table: &TableData,
    x_col: usize,
    y_col: usize,
    max_degree: usize,
) -> Result<StatsLabResult, String> {
    let n = table.n_rows();
    let p = table.n_cols();
    if n < 5 {
        return Err("Need at least 5 rows for polynomial fit".into());
    }
    if p < 2 {
        return Err("Need at least 2 numeric columns (X and y)".into());
    }
    let y_col = y_col.min(p - 1);
    let xc = if x_col == y_col {
        (0..p).find(|&i| i != y_col).unwrap_or(0)
    } else {
        x_col
    };
    let x_name = col_name(table, xc);
    let y_name = col_name(table, y_col);
    let xv = table.columns[xc].clone();
    let yv = table.columns[y_col].clone();
    let max_degree = max_degree.clamp(1, 8);

    let search = fit_polynomial_degrees_with(
        &xv,
        &yv,
        &PolynomialSearchConfig {
            max_degree,
            return_residuals: true,
            print_warnings: false,
        },
    )
    .map_err(|e| format!("Polyreg: {e:?}"))?;

    if search.fits.is_empty() {
        return Err("No feasible polynomial degree for this sample size".into());
    }

    // Prefer AIC for ★ (R² alone almost always picks max degree).
    let best_aic_d = search.best_degree_by_aic().unwrap_or(search.fits[0].degree);
    let best_r2_d = search.best_degree_by_r2().unwrap_or(search.fits[0].degree);
    let best_bic_d = search.best_degree_by_bic().unwrap_or(search.fits[0].degree);
    let mut best_idx = 0usize;
    let mut rows: Vec<SplitMetricsRow> = Vec::with_capacity(search.fits.len());

    for (i, f) in search.fits.iter().enumerate() {
        let is_best = f.degree == best_aic_d;
        if is_best {
            best_idx = i;
        }
        let y_hat = f.predict(&xv);
        let y_hat_v: Vec<f64> = y_hat.to_vec();
        let res = residuals(&yv, &y_hat_v);
        let ba_mean: Vec<f64> = yv
            .iter()
            .zip(y_hat_v.iter())
            .map(|(&a, &p)| (a + p) / 2.0)
            .collect();
        // Sorted curve for overlay (x vs ŷ)
        let mut pairs: Vec<(f64, f64)> = xv
            .iter()
            .zip(y_hat_v.iter())
            .map(|(&x, &yh)| (x, yh))
            .collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let fit_x: Vec<f64> = pairs.iter().map(|p| p.0).collect();
        let fit_yh: Vec<f64> = pairs.iter().map(|p| p.1).collect();

        let beta = f.coeffs_packed();
        let beta_s: String = beta
            .iter()
            .enumerate()
            .map(|(j, b)| format!("β{j}={b:.3}"))
            .collect::<Vec<_>>()
            .join(" ");

        // Nested χ² vs previous degree (is the extra term justified?)
        let chi_note = match (f.chi2_vs_prev, f.chi2_vs_prev_p, f.chi2_vs_prev_df) {
            (Some(chi), Some(p), df) if df > 0 => {
                format!("χ²Δ={chi:.3} (df={df}, p={p:.3})")
            }
            (Some(chi), None, df) if df > 0 => format!("χ²Δ={chi:.3} (df={df})"),
            _ => "χ²Δ=— (base)".into(),
        };

        // Columns for Poly table: R² | adjR² | AIC  (RMSE moved to note with χ²)
        rows.push(SplitMetricsRow {
            label: if is_best {
                format!("d={}★", f.degree)
            } else if f.degree == best_r2_d && best_r2_d != best_aic_d {
                format!("d={}☆", f.degree) // max R² but not preferred
            } else {
                format!("d={}", f.degree)
            },
            n,
            r2: f.report.r2,
            rmse: f.scores.adj_r2, // column header becomes adjR² for poly
            mae: f.scores.aic,     // column header becomes AIC for poly
            metric_kind: SplitMetricKind::Regression,
            note: format!(
                "{chi_note}  RMSE={:.4}  BIC={:.2}  {beta_s}",
                f.report.rmse, f.scores.bic
            ),
            is_best,
            plot_x: xv.clone(),
            plot_y: yv.clone(),
            fit_line_x: fit_x,
            fit_line_y: fit_yh,
            ba_mean,
            residuals: res,
            scatter_x_label: x_name.clone(),
            scatter_y_label: y_name.clone(),
            is_pred_vs_obs: false,
        });
    }

    let best = &rows[best_idx];
    let best_footer = format!(
        "★ best by AIC: {}  R²={:.4}  adjR²={:.4}  AIC={:.2}\n  {}\n  \
         ☆ max R² = d={}  ·  min BIC = d={}  ·  tiny R² gains often fail χ² / AIC",
        best.label.replace('★', "").replace('☆', ""),
        best.r2,
        best.rmse,
        best.mae,
        best.note,
        best_r2_d,
        best_bic_d,
    );

    let mut r = empty_result(StatsLabTask::Poly);
    r.model_label = format!("Poly max d={max_degree}");
    r.summary = format!(
        "Polyreg · y={y_name}  X={x_name}  max_d={max_degree}  ·  {} degrees\n  \
         ★ = min AIC (preferred)  ·  ☆ = max R² if different  ·  χ²Δ = LR vs d−1",
        rows.len()
    );
    r.interpretation = format!(
        "In-sample R² always rises with degree; use AIC/BIC and nested χ². \
         If χ²Δ p ≫ 0.05, the extra term is not justified. ↑↓/f focus · d/D max degree."
    );
    if !search.warnings.is_empty() {
        r.interpretation.push_str(" · ");
        r.interpretation.push_str(&search.warnings[0]);
    }
    r.metrics_rows = rows;
    r.focused_row = best_idx;
    r.best_row = Some(best_idx);
    r.table_footer = best_footer;
    r.metrics_table_title = "Degrees".into();
    seed_active_from_focus(&mut r);
    Ok(r)
}

/// Logistic classification: binary or multiclass OVR when y has ≥3 levels.
fn run_classify(table: &TableData, x_col: usize, y_col: usize) -> Result<StatsLabResult, String> {
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
fn encode_class_labels(y: &[f64]) -> Result<(Vec<usize>, Vec<f64>), String> {
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

fn format_conf_mat(cm: &ndarray::Array2<usize>, class_values: &[f64]) -> String {
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

fn format_per_class_prf(rep: &symworx_stats::ClassificationReport, class_values: &[f64]) -> String {
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

fn logistic_config() -> LogisticConfig {
    LogisticConfig {
        max_iter: 4000,
        learning_rate: 0.25,
        ..Default::default()
    }
}

/// Multi-X OLS **or** logistic + train/test + optional k-fold; metrics table drives plots.
fn run_pipeline(
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

fn run_pipeline_ols(
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

fn run_pipeline_logistic(
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
fn fit_predict_clf_row(
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

fn fit_on_train_score_val_clf(
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

fn fold_mean_sd(vals: &[f64], name: &str) -> String {
    if vals.is_empty() {
        return "no CV folds".into();
    }
    let mean_f = vals.iter().sum::<f64>() / vals.len() as f64;
    let var = vals.iter().map(|v| (v - mean_f).powi(2)).sum::<f64>() / vals.len() as f64;
    format!(
        "CV {}-fold mean {name}={mean_f:.4}  sd={:.4}",
        vals.len(),
        var.sqrt()
    )
}

fn seed_active_from_focus(r: &mut StatsLabResult) {
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

fn metrics_row_reg(
    label: &str,
    note: &str,
    y_obs: &[f64],
    y_hat: &[f64],
    y_name: &str,
) -> SplitMetricsRow {
    let n = y_obs.len().min(y_hat.len());
    let y_obs = &y_obs[..n];
    let y_hat = &y_hat[..n];
    let res = residuals(y_obs, y_hat);
    let r2v = r2(y_obs, y_hat);
    let rmsev = rmse(y_obs, y_hat);
    let maev = mae(y_obs, y_hat);
    let ba: Vec<f64> = y_obs
        .iter()
        .zip(y_hat.iter())
        .map(|(&a, &p)| (a + p) / 2.0)
        .collect();
    let mut lo = y_obs
        .iter()
        .chain(y_hat.iter())
        .copied()
        .fold(f64::INFINITY, f64::min);
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

fn rows_at(x: &Array2<f64>, idx: &[usize]) -> Array2<f64> {
    let p = x.ncols();
    let mut out = Array2::zeros((idx.len(), p));
    for (r, &i) in idx.iter().enumerate() {
        out.row_mut(r).assign(&x.row(i));
    }
    out
}

fn array1_at(y: &Array1<f64>, idx: &[usize]) -> Array1<f64> {
    Array1::from(idx.iter().map(|&i| y[i]).collect::<Vec<_>>())
}
