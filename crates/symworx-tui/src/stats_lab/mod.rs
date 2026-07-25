// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! StatsSym Lab runners: describe, correlate, OLS/poly fit, classify, pipeline.

mod classify;
mod describe;
mod pipeline;
mod poly;
mod regress;
mod util;

use classify::run_classify;
use describe::{
    run_correlate,
    run_describe,
};
use pipeline::run_pipeline;
use poly::run_poly;
use regress::run_regress;
use symworx_io::TableData;

use crate::app::{
    PipelineModel,
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
) -> Result<crate::app::StatsLabResult, String> {
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
