// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Polynomial degree-search Lab task.

use symworx_io::TableData;
use symworx_stats::{
    fit_polynomial_degrees_with,
    residuals,
    PolynomialSearchConfig,
};

use super::util::{
    col_name,
    empty_result,
    seed_active_from_focus,
};
use crate::app::{
    SplitMetricKind,
    SplitMetricsRow,
    StatsLabResult,
    StatsLabTask,
};

pub fn run_poly(
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
        best.label.replace(['★', '☆'], ""),
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
    r.interpretation = "In-sample R² always rises with degree; use AIC/BIC and nested χ². \
         If χ²Δ p ≫ 0.05, the extra term is not justified. ↑↓/f focus · d/D max degree.".to_string();
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
