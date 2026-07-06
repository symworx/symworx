// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Recurrence Quantification Analysis (RQA)
//!
//! Tools for constructing recurrence plots and quantifying the
//! nonlinear dynamics of time series via recurrence quantification
//! analysis (RQA). Cross-recurrence (CRQA) support is planned.
//!
//! ## Quick start
//! ```ignore
//! use symworx_dynamics::rqa::{rqa, RecurrencePlot};
//!
//! let series: Vec<f64> = /* your data */;
//! let result = rqa(&series, 3, 1, 0.5, 1);
//! println!("DET = {}", result.determinism);
//!
//! let rp = RecurrencePlot::from_series(&series, 3, 1, 0.5, 1);
//! ```

mod metrics;
mod plot;
mod utils;

pub use metrics::{DEFAULT_LMIN, DEFAULT_VMIN, RqaResult, rqa, rqa_from_trajectory};
pub use plot::RecurrencePlot;

// TODO (Phase 4): pub use metrics::crqa; + CrossRecurrencePlot + CrqaResult
