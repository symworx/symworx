// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Deconvolution algorithms for signal restoration.
//!
//! This module provides tools to reverse the effects of convolution
//! (e.g. sensor response, filtering, diffusion) on physiological signals.

pub mod wiener;
pub mod nnls;
pub mod utils;

pub use wiener::wiener_deconvolution;
pub use nnls::nonnegative_deconvolution;
