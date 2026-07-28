// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Describe + correlate Lab tasks.

use symworx_io::TableData;
use symworx_stats::{
    mean,
    median,
    std_dev_sample,
};

use super::util::{
    col_name,
    empty_result,
};
use crate::app::{
    StatsLabResult,
    StatsLabTask,
};

pub fn run_describe(table: &TableData, col: usize) -> Result<StatsLabResult, String> {
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
    let interpretation = format!("Column “{name}”: centre ≈ {m:.3} (median {med:.3}); spread sd ≈ {s:.3}.");
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

pub fn run_correlate(table: &TableData, x_col: usize, y_col: usize) -> Result<StatsLabResult, String> {
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
