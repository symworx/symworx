// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

/// One row-major trajectory.
///
/// `points.len()` must be `n * dim`. Row `i` occupies
/// `points[i * dim .. (i + 1) * dim]`.
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
    pub nrrows: usize,
    /// Number of columns (`right.n`).
    pub nccols: usize,
    /// Packed bits, `nrrows * nccols.div_ceil(8)` bytes.
    pub bits: alloc::vec::Vec<u8>,
}

impl RecurrenceMask {
    /// Byte stride between rows.
    pub fn row_stride(&self) -> usize {
        self.nccols.div_ceil(8)
    }

    /// `true` when pair `(row, col)` is recurrent.
    ///
    /// Out-of-range indexes are `false`.
    pub fn bit(&self, row: usize, col: usize) -> bool {
        if row >= self.nrrows || col >= self.nccols {
            return false;
        }
        let byte = self.bits[row * self.row_stride() + col / 8];
        (byte & (1u8 << (col % 8))) != 0
    }
}

/// Scalar recurrence mask. Matches the specification in `symworx-gpu`.
pub fn recurrence_mask(
    left: Trajectory,
    right: Trajectory,
    radius: f64,
    theiler: usize,
) -> Result<RecurrenceMask, crate::NpuError> {
    prepare(left, right, radius)?;
    if left.n == 0 || right.n == 0 {
        return Ok(RecurrenceMask {
            nrrows: left.n,
            nccols: right.n,
            bits: alloc::vec::Vec::new(),
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

pub fn prepare(left: Trajectory, right: Trajectory, radius: f64) -> Result<(), crate::NpuError> {
    check_layout(left)?;
    check_layout(right)?;
    if left.n > 0 && right.n > 0 && left.dim != right.dim {
        return Err(crate::NpuError::InvalidInput("trajectory dimensions differ"));
    }
    if !radius.is_finite() {
        return Err(crate::NpuError::InvalidInput("radius must be finite"));
    }
    Ok(())
}

pub fn blank_mask(nrrows: usize, nccols: usize) -> Result<RecurrenceMask, crate::NpuError> {
    if nrrows == 0 || nccols == 0 {
        return Ok(RecurrenceMask {
            nrrows,
            nccols,
            bits: alloc::vec::Vec::new(),
        });
    }
    let stride = nccols.div_ceil(8);
    let bytes = nrrows.checked_mul(stride).ok_or(crate::NpuError::Overflow)?;
    Ok(RecurrenceMask {
        nrrows,
        nccols,
        bits: alloc::vec![0u8; bytes],
    })
}

fn check_layout(traj: Trajectory) -> Result<(), crate::NpuError> {
    let cells = traj.n.checked_mul(traj.dim).ok_or(crate::NpuError::Overflow)?;
    if traj.points.len() != cells {
        return Err(crate::NpuError::InvalidInput("points length must equal n * dim"));
    }
    if traj.n > 0 && traj.dim == 0 {
        return Err(crate::NpuError::InvalidInput("dim must be non-zero"));
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
        recurrence_mask(
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
        let err = recurrence_mask(
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
            crate::NpuError::InvalidInput("points length must equal n * dim")
        ));

        let mismatch = recurrence_mask(
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
            crate::NpuError::InvalidInput("trajectory dimensions differ")
        ));

        let empty = recurrence_mask(
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
        assert_eq!(empty.nrrows, 0);
        assert!(empty.bits.is_empty());

        let non_finite = recurrence_mask(
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
        assert!(matches!(
            non_finite,
            crate::NpuError::InvalidInput("radius must be finite")
        ));
    }
}
