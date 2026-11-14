// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! # symworx-gpu
//!
//! Opt-in device kernels for SymWorx.
//!
//! The default build is pure Rust: a CPU recurrence mask, a CPU order-1 DFA
//! fluctuation, and no HIP runtime. The `hip` feature compiles
//! `kernels/recurrence.hip` and `kernels/dfa.hip` with `hipcc` and links them
//! for either AMD (`HIP_PLATFORM=amd`) or NVIDIA (`HIP_PLATFORM=nvidia`).
//!
//! Callers in `symworx-dynamics` keep the existing CPU recurrence plot and the
//! CPU DFA estimator. Device entry points are separate and return
//! [`GpuError::Unavailable`] when this crate was built without `hip` or no
//! device is present. The DFA kernel returns `F(n)` for box lengths the host
//! chose. The log-log slope stays on the CPU.

#![warn(missing_docs)]

mod dfa;
mod error;
mod hardware;
mod recurrence;

#[cfg(feature = "hip")]
mod device;

pub use dfa::{
    dfa_fluctuation,
    dfa_fluctuation_cpu,
};
pub use error::{
    GpuError,
    MAX_BOOL_MATRIX_CELLS,
    bool_matrix_too_large,
};
pub use hardware::{
    Hardware,
    HipDevice,
    HipTarget,
    PciGpu,
    Vendor,
    detect_hardware,
};
pub use recurrence::{
    DeviceInfo,
    RecurrenceMask,
    Trajectory,
    probe,
    recurrence_mask,
    recurrence_mask_cpu,
};
