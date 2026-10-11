// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Entropy measures.
//!
//! Sample entropy, multiscale entropy, histogram mutual information,
//! and discrete transfer entropy (bivariate, joint multi-source, and
//! conditional). Pre-binned series use `transfer_entropy_discrete*` and
//! `mutual_information_discrete`; continuous series are quantile-binned.

mod multiscale_entropy;
mod mutual_information;
mod sample_entropy;
mod transfer_entropy;

pub use multiscale_entropy::multiscale_entropy;
pub use mutual_information::{
    DEFAULT_AMI_BINS,
    ami,
    cross_ami,
    first_local_minimum,
    mutual_information,
    mutual_information_discrete,
};
pub use sample_entropy::sample_entropy;
pub use transfer_entropy::{
    TeConfig,
    transfer_entropy,
    transfer_entropy_conditional,
    transfer_entropy_discrete,
    transfer_entropy_discrete_conditional,
    transfer_entropy_discrete_mv,
    transfer_entropy_mv,
    transfer_entropy_with,
};
