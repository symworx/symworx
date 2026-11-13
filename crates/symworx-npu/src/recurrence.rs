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
///
/// `|i - j| <= theiler` stays unset, including the diagonal when `theiler` is 0.
pub fn recurrence_mask(
    left: Trajectory,
    right: Trajectory,
    radius: f64,
    theiler: usize,
) -> Result<RecurrenceMask, crate::NpuError> {
    fill_mask(left, right, radius, theiler, false)
}

/// Delay embedding. Row `i` is `series[i + k * tau]` for `k` in `0..m`.
///
/// The row count is `series.len() - (m - 1) * tau`. This matches `edim` in
/// `symworx-dynamics`.
pub fn embed(series: &[f64], m: usize, tau: usize) -> Result<alloc::vec::Vec<f64>, crate::NpuError> {
    if m == 0 || tau == 0 {
        return Err(crate::NpuError::InvalidInput("embedding m and tau must be non-zero"));
    }
    let span = (m - 1)
        .checked_mul(tau)
        .and_then(|lag| lag.checked_add(1))
        .ok_or(crate::NpuError::Overflow)?;
    if series.len() < span {
        return Err(crate::NpuError::InvalidInput("series is shorter than the embedding"));
    }
    let n = series.len() - (m - 1) * tau;
    let cells = n.checked_mul(m).ok_or(crate::NpuError::Overflow)?;
    let mut points = alloc::vec::Vec::with_capacity(cells);
    for i in 0..n {
        for k in 0..m {
            points.push(series[i + k * tau]);
        }
    }
    Ok(points)
}

/// Cross-recurrence mask of two scalar series.
///
/// Both series use the same `m` and `tau`. Distance is squared Euclidean.
/// Pairs with `|i - j| <= theiler` are skipped only when `theiler > 0`, so a
/// window of 0 keeps the simultaneous pair. [`recurrence_mask`] still clears
/// `|i - j| == 0`.
pub fn cross_recurrence_mask(
    x: &[f64],
    y: &[f64],
    m: usize,
    tau: usize,
    radius: f64,
    theiler: usize,
) -> Result<RecurrenceMask, crate::NpuError> {
    let x_points = embed(x, m, tau)?;
    let y_points = embed(y, m, tau)?;
    let nx = x_points.len() / m;
    let ny = y_points.len() / m;
    fill_mask(
        Trajectory {
            points: &x_points,
            n: nx,
            dim: m,
        },
        Trajectory {
            points: &y_points,
            n: ny,
            dim: m,
        },
        radius,
        theiler,
        true,
    )
}

fn fill_mask(
    left: Trajectory,
    right: Trajectory,
    radius: f64,
    theiler: usize,
    keep_diagonal_at_zero: bool,
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
            let close = (i as isize - j as isize).unsigned_abs() <= theiler;
            let skip = if keep_diagonal_at_zero {
                theiler > 0 && close
            } else {
                close
            };
            if skip {
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

    #[test]
    fn embed_uses_the_host_delay() {
        let points = embed(&[0.0, 1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
        assert_eq!(points, vec![0.0, 2.0, 1.0, 3.0, 2.0, 4.0]);
        assert!(embed(&[0.0, 1.0], 0, 1).is_err());
        assert!(embed(&[0.0, 1.0], 2, 0).is_err());
        assert!(embed(&[0.0, 1.0], 3, 1).is_err());
    }

    #[test]
    fn cross_mask_keeps_the_simultaneous_pair_when_theiler_is_zero() {
        let x = [0.0, 5.0, 0.0];
        let y = [0.0, 5.0];
        let cross = cross_recurrence_mask(&x, &y, 1, 1, 0.1, 0).unwrap();
        assert!(cross.bit(0, 0));
        assert!(cross.bit(1, 1));
        assert!(cross.bit(2, 0));
        assert!(!cross.bit(0, 1));
        assert!(!cross.bit(1, 0));
        assert!(!cross.bit(2, 1));

        let auto = recurrence_mask(
            Trajectory {
                points: &x,
                n: 3,
                dim: 1,
            },
            Trajectory {
                points: &y,
                n: 2,
                dim: 1,
            },
            0.1,
            0,
        )
        .unwrap();
        assert!(!auto.bit(0, 0));
        assert!(!auto.bit(1, 1));

        let cleared = cross_recurrence_mask(&x, &y, 1, 1, 0.1, 1).unwrap();
        assert!(!cleared.bit(0, 0));
        assert!(!cleared.bit(1, 1));
        assert!(cleared.bit(2, 0));
    }
}
