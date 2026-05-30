// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Gait modeling and analysis.
//!
//! Core types for gait simulation parameters (`GaitParams`) and derived
//! spatiotemporal metrics (`GaitData`).
//!
//! Provides stride/step timing, cadence, length calculations, and basic
//! vertical oscillation analysis. Designed for use with RQA and nonlinear
//! dynamics tooling in `symworx-dynamics`.
//!
//! ## Example
//! ```ignore
//! use symworx_biosym::GaitParams;
//! use symworx_biosym::biomechanics::gait::GaitData;
//!
//! let params = GaitParams::default().with_defaults();
//! let mut data = GaitData::new(100.0);
//! data.stride_times = Some(ndarray::array![0.0, 1.0, 2.0]);
//! let _ = data.calculate_stride_intervals();
//! ```

mod data;
mod metrics;
mod params;

pub use data::GaitData;
pub use params::GaitParams;
