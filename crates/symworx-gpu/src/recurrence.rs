// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Packed recurrence masks.
//!
//! A pair is recurrent when the embedding dimensions match, `|i - j| > theiler`,
//! and the squared Euclidean distance is `<= radius²`. A negative finite radius
//! yields an all-false mask. Non-finite radii are rejected. NaN coordinates
//! are non-recurrent.

use crate::error::GpuError;

/// One row-major trajectory.
///
/// `points.len()` must be `n * dim`. Row `i` occupies `points[i * dim .. (i + 1) * dim]`.
#[derive(Debug, Clone, Copy)]
pub struct Trajectory<'a> {
    /// Row-major coordinates.
    pub points: &'a [f64],
    /// Number of rows.
    pub n: usize,
    /// Coordinates per row.
    pub dim: usize,
}

/// Row-major bit matrix. Bit 0 of each byte is the lowest column in that byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceMask {
    /// Number of rows (`left.n`).
    pub nrows: usize,
    /// Number of columns (`right.n`).
    pub ncols: usize,
    /// Packed bits, `nrows * ncols.div_ceil(8)` bytes.
    pub bits: Vec<u8>,
}

impl RecurrenceMask {
    /// Byte stride between rows.
    pub fn row_stride(&self) -> usize {
        self.ncols.div_ceil(8)
    }

    /// `true` when pair `(row, col)` is recurrent.
    ///
    /// Out-of-range indexes are `false`.
    pub fn bit(&self, row: usize, col: usize) -> bool {
        if row >= self.nrows || col >= self.ncols {
            return false;
        }
        let byte = self.bits[row * self.row_stride() + col / 8];
        (byte & (1u8 << (col % 8))) != 0
    }
}

/// Device selected for a HIP build.
///
/// `probe` is `None` when the `hip` feature is off or the runtime has no device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Selected device index.
    pub index: i32,
    /// Devices reported by the runtime.
    pub count: i32,
    /// Runtime device name.
    pub name: String,
    /// `amd` or `nvidia`, from `HIP_PLATFORM` at build time.
    pub platform: &'static str,
}

/// CPU recurrence mask. This is the specification the device kernel matches.
pub fn recurrence_mask_cpu(
    left: Trajectory<'_>,
    right: Trajectory<'_>,
    radius: f64,
    theiler: usize,
) -> Result<RecurrenceMask, GpuError> {
    prepare(left, right, radius, theiler)?;
    if left.n == 0 || right.n == 0 {
        return Ok(RecurrenceMask {
            nrows: left.n,
            ncols: right.n,
            bits: Vec::new(),
        });
    }
    if radius < 0.0 {
        return blank_mask(left.n, right.n);
    }

    let radius2 = radius * radius;
    let mut mask = blank_mask(left.n, right.n)?;
    let stride = mask.row_stride();
    for i in 0..left.n {
        let a = row(left, i);
        for j in 0..right.n {
            if (i as isize - j as isize).unsigned_abs() <= theiler {
                continue;
            }
            if squared_distance(a, row(right, j)) <= radius2 {
                let byte = i * stride + j / 8;
                mask.bits[byte] |= 1u8 << (j % 8);
            }
        }
    }
    Ok(mask)
}

/// Device recurrence mask.
///
/// Without the `hip` feature this returns [`GpuError::Unavailable`].
/// With `hip`, it uploads `f64` rows, launches the HIP kernel, and copies the
/// packed mask back. The same host slice for both sides is uploaded once.
pub fn recurrence_mask(
    left: Trajectory<'_>,
    right: Trajectory<'_>,
    radius: f64,
    theiler: usize,
) -> Result<RecurrenceMask, GpuError> {
    #[cfg(not(feature = "hip"))]
    {
        let _ = (left, right, radius, theiler);
        Err(GpuError::Unavailable)
    }
    #[cfg(feature = "hip")]
    {
        crate::device::recurrence_mask_hip(left, right, radius, theiler)
    }
}

/// Runtime probe. `None` when HIP was not compiled in, or no device is usable.
pub fn probe() -> Option<DeviceInfo> {
    #[cfg(not(feature = "hip"))]
    {
        None
    }
    #[cfg(feature = "hip")]
    {
        crate::device::probe_hip().ok()
    }
}

pub fn prepare(left: Trajectory<'_>, right: Trajectory<'_>, radius: f64, theiler: usize) -> Result<(), GpuError> {
    check_layout(left)?;
    check_layout(right)?;
    if left.n > 0 && right.n > 0 && left.dim != right.dim {
        return Err(GpuError::InvalidInput("trajectory dimensions differ"));
    }
    if !radius.is_finite() {
        return Err(GpuError::InvalidInput("radius must be finite"));
    }
    if theiler > i32::MAX as usize {
        return Err(GpuError::InvalidInput("theiler does not fit in i32"));
    }
    Ok(())
}

pub fn blank_mask(nrows: usize, ncols: usize) -> Result<RecurrenceMask, GpuError> {
    if nrows == 0 || ncols == 0 {
        return Ok(RecurrenceMask {
            nrows,
            ncols,
            bits: Vec::new(),
        });
    }
    let stride = ncols.div_ceil(8);
    let bytes = nrows
        .checked_mul(stride)
        .ok_or(GpuError::InvalidInput("packed mask length overflows"))?;
    if bytes > i32::MAX as usize {
        return Err(GpuError::InvalidInput("packed mask does not fit in an i32 byte length"));
    }
    Ok(RecurrenceMask {
        nrows,
        ncols,
        bits: vec![0u8; bytes],
    })
}

fn check_layout(traj: Trajectory<'_>) -> Result<(), GpuError> {
    let cells = traj
        .n
        .checked_mul(traj.dim)
        .ok_or(GpuError::InvalidInput("trajectory length overflows"))?;
    if traj.points.len() != cells {
        return Err(GpuError::InvalidInput("points length must equal n * dim"));
    }
    if traj.n > 0 && traj.dim == 0 {
        return Err(GpuError::InvalidInput("dim must be non-zero"));
    }
    if traj.n > i32::MAX as usize || traj.dim > i32::MAX as usize {
        return Err(GpuError::InvalidInput("trajectory shape does not fit in i32"));
    }
    Ok(())
}

fn row(traj: Trajectory<'_>, i: usize) -> &[f64] {
    let start = i * traj.dim;
    &traj.points[start..start + traj.dim]
}

fn squared_distance(a: &[f64], b: &[f64]) -> f64 {
    let mut acc = 0.0;
    for (x, y) in a.iter().zip(b) {
        let d = x - y;
        acc += d * d;
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(
        left: &[f64],
        n: usize,
        dim: usize,
        right: &[f64],
        m: usize,
        radius: f64,
        theiler: usize,
    ) -> RecurrenceMask {
        recurrence_mask_cpu(
            Trajectory { points: left, n, dim },
            Trajectory {
                points: right,
                n: m,
                dim,
            },
            radius,
            theiler,
        )
        .unwrap()
    }

    #[test]
    fn square_mask_uses_squared_distance_and_theiler() {
        // Rows [0] and [3]. radius 3 includes the off-diagonal. theiler 0 drops the diagonal.
        let points = [0.0, 3.0];
        let got = mask(&points, 2, 1, &points, 2, 3.0, 0);
        assert!(!got.bit(0, 0));
        assert!(got.bit(0, 1));
        assert!(got.bit(1, 0));
        assert!(!got.bit(1, 1));
        assert_eq!(got.bits, vec![0b0000_0010, 0b0000_0001]);
    }

    #[test]
    fn rectangular_mask_packs_column_eight_in_the_next_byte() {
        let left = [0.0];
        let mut right = [100.0; 9];
        right[8] = 0.0;
        let got = mask(&left, 1, 1, &right, 9, 0.1, 0);
        assert_eq!(got.row_stride(), 2);
        assert!(!got.bit(0, 0));
        assert!(!got.bit(0, 7));
        assert!(got.bit(0, 8));
        assert_eq!(got.bits, vec![0, 0b0000_0001]);
    }

    #[test]
    fn theiler_window_drops_near_diagonals() {
        let points = [0.0, 0.0, 0.0];
        let got = mask(&points, 3, 1, &points, 3, 1.0, 1);
        assert!(!got.bit(0, 1));
        assert!(got.bit(0, 2));
        assert!(got.bit(2, 0));
    }

    #[test]
    fn negative_radius_is_all_false_and_nan_is_not_recurrent() {
        let points = [0.0, 1.0];
        let neg = mask(&points, 2, 1, &points, 2, -0.5, 0);
        assert_eq!(neg.bits, vec![0, 0]);

        let nan = [0.0, f64::NAN];
        let got = mask(&points, 2, 1, &nan, 2, 10.0, 0);
        assert!(!got.bit(0, 1));
        assert!(got.bit(1, 0));
    }

    #[test]
    fn rejects_bad_shapes_and_non_finite_radius() {
        let err = recurrence_mask_cpu(
            Trajectory {
                points: &[0.0, 1.0],
                n: 2,
                dim: 1,
            },
            Trajectory {
                points: &[0.0],
                n: 1,
                dim: 2,
            },
            1.0,
            0,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            GpuError::InvalidInput("points length must equal n * dim")
        ));

        let mismatch = recurrence_mask_cpu(
            Trajectory {
                points: &[0.0, 1.0],
                n: 1,
                dim: 2,
            },
            Trajectory {
                points: &[0.0],
                n: 1,
                dim: 1,
            },
            1.0,
            0,
        )
        .unwrap_err();
        assert!(matches!(
            mismatch,
            GpuError::InvalidInput("trajectory dimensions differ")
        ));

        let empty = recurrence_mask_cpu(
            Trajectory {
                points: &[],
                n: 0,
                dim: 2,
            },
            Trajectory {
                points: &[0.0, 1.0],
                n: 1,
                dim: 2,
            },
            1.0,
            0,
        )
        .unwrap();
        assert_eq!(empty.nrows, 0);
        assert!(empty.bits.is_empty());

        let non_finite = recurrence_mask_cpu(
            Trajectory {
                points: &[0.0],
                n: 1,
                dim: 1,
            },
            Trajectory {
                points: &[0.0],
                n: 1,
                dim: 1,
            },
            f64::NAN,
            0,
        )
        .unwrap_err();
        assert!(matches!(non_finite, GpuError::InvalidInput("radius must be finite")));
    }

    #[cfg(not(feature = "hip"))]
    #[test]
    fn default_build_has_no_device() {
        assert!(probe().is_none());
        let err = recurrence_mask(
            Trajectory {
                points: &[0.0],
                n: 1,
                dim: 1,
            },
            Trajectory {
                points: &[0.0],
                n: 1,
                dim: 1,
            },
            1.0,
            0,
        )
        .unwrap_err();
        assert!(matches!(err, GpuError::Unavailable));
    }

    #[test]
    fn bool_cap_is_exclusive_of_the_square_limit() {
        use crate::{
            MAX_BOOL_MATRIX_CELLS,
            bool_matrix_too_large,
        };

        assert!(!bool_matrix_too_large(20_000, 20_000));
        assert!(bool_matrix_too_large(20_001, 20_000));
        assert_eq!(MAX_BOOL_MATRIX_CELLS, 400_000_000);
    }
}
