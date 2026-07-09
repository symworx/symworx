// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Recurrence Quantification Analysis (RQA)
//!
//! Tools for constructing recurrence plots and quantifying the
//! nonlinear dynamics of time series via recurrence quantification
//! analysis (RQA). Includes cross-recurrence (cRQA) between two series.
//!
//! ## Quick start
//! ```ignore
//! use symworx_dynamics::rqa::{rqa, crqa, RecurrencePlot};
//!
//! let series: Vec<f64> = /* your data */;
//! let result = rqa(&series, 3, 1, 0.5, 1);
//! println!("DET = {}", result.determinism);
//!
//! // cRQA between two (possibly different length) series
//! let other: Vec<f64> = /* ... */;
//! let _cres = crqa(&series, &other, 3, 1, 0.5, 0);
//!
//! let rp = RecurrencePlot::from_series(&series, 3, 1, 0.5, 1);
//! ```

mod metrics;
mod plot;
mod utils;

pub use metrics::{crqa, DEFAULT_LMIN, DEFAULT_VMIN, RqaResult, rqa, rqa_from_trajectory};
pub use plot::{CrossRecurrencePlot, RecurrencePlot};
