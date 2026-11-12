// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Deconvolution algorithms for signal restoration.
//!
//! Reverse the effects of convolution (sensor response, elimination kernels)
//! on physiological signals. Hormone reconstruction uses
//! [`nonnegative_deconvolution`] with [`exponential_kernel`].

pub mod kernel;
pub mod nnls;
#[path = "weiner.rs"]
pub mod wiener;

pub use kernel::{
    convolve,
    exponential_kernel,
};
pub use nnls::{
    burst_indices,
    gh_burst_deconvolve,
    gh_deconvolve,
    gh_deconvolve_sparse,
    gh_reconvolve,
    nonnegative_deconvolution,
    nonnegative_deconvolution_on_support,
    sparse_nonnegative_deconvolution,
};
pub use wiener::wiener_deconvolution;
