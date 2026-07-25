// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Simple OLS regression Lab task.

use ndarray::{
    Array1,
    Array2,
};
use symworx_io::TableData;
use symworx_stats::{
    mae,
    ols,
    r2,
    regression_report,
    residuals,
    rmse,
};

use super::util::{
    col_name,
    empty_result,
};
use crate::app::{
    StatsLabResult,
    StatsLabTask,
};

pub(crate) fn run_regress(
    table: &TableData,
    x_col: usize,
    y_col: usize,
) -> Result<StatsLabResult, String> {
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
