// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Errors for device kernels. These stay in this crate and do not join `SymError`.

/// Largest `nrows * ncols` expanded into an `Array2<bool>` by dynamics.
///
/// A packed mask is one bit per pair. The boolean matrix is one byte per pair,
/// about 400 MiB at this cap (`20_000²`).
pub const MAX_BOOL_MATRIX_CELLS: usize = 20_000 * 20_000;

/// `true` when a boolean matrix of this shape would pass [`MAX_BOOL_MATRIX_CELLS`].
pub fn bool_matrix_too_large(nrows: usize, ncols: usize) -> bool {
    match nrows.checked_mul(ncols) {
        Some(cells) => cells > MAX_BOOL_MATRIX_CELLS,
        None => true,
    }
}

/// Failure from a GPU entry point.
#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    /// Built without the `hip` feature, or the HIP runtime reports no device.
    #[error("no HIP device is available")]
    Unavailable,

    /// Shape, radius, or device-index problem in the request.
    #[error("{0}")]
    InvalidInput(&'static str),

    /// Expanding the packed mask to one byte per pair would pass the cap.
    #[error("boolean recurrence matrix {nrows} x {ncols} exceeds {max} cells")]
    TooLarge {
        /// Rows in the mask.
        nrows: usize,
        /// Columns in the mask.
        ncols: usize,
        /// [`MAX_BOOL_MATRIX_CELLS`].
        max: usize,
    },

    /// A HIP runtime call failed.
    #[error("HIP call {call} failed with status {status}")]
    Device {
        /// HIP error code, or a negative code from the launcher.
        status: i32,
        /// Launcher or runtime entry that returned `status`.
        call: &'static str,
    },
}
