// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Device recurrence plots.
//!
//! The CPU constructors are unchanged. These methods return
//! [`GpuError::Unavailable`] when `symworx-gpu` has no HIP device, including
//! a build without the `hip` feature. They do not fall back to the CPU loop.

use std::borrow::Cow;

use ndarray::Array2;
use symworx_gpu::{
    GpuError,
    MAX_BOOL_MATRIX_CELLS,
    RecurrenceMask,
    Trajectory,
    bool_matrix_too_large,
    recurrence_mask,
};

use super::plot::{
    CrossRecurrencePlot,
    RecurrencePlot,
};

impl RecurrencePlot {
    /// Recurrence plot from a device mask.
    ///
    /// `trajectory` rows are embedded points. A non-standard layout is copied
    /// into row-major order. An empty trajectory returns an empty plot.
    /// A non-finite radius is [`GpuError::InvalidInput`]. A mask larger than
    /// [`MAX_BOOL_MATRIX_CELLS`] is [`GpuError::TooLarge`].
    pub fn try_from_trajectory_on_device(
        trajectory: &Array2<f64>,
        radius: f64,
        theiler: usize,
    ) -> Result<Self, GpuError> {
        let matrix = device_matrix(trajectory, trajectory, radius, theiler)?;
        if matrix.is_empty() {
            return Ok(Self::new());
        }
        let n_points = matrix.nrows();
        Ok(Self {
            matrix,
            radius,
            n_points,
        })
    }
}

impl CrossRecurrencePlot {
    /// Cross-recurrence plot from a device mask.
    ///
    /// Dimension mismatch and empty inputs return an empty plot, matching
    /// [`CrossRecurrencePlot::from_trajectories`].
    pub fn try_from_trajectories_on_device(
        tx: &Array2<f64>,
        ty: &Array2<f64>,
        radius: f64,
        theiler: usize,
    ) -> Result<Self, GpuError> {
        let matrix = device_matrix(tx, ty, radius, theiler)?;
        if matrix.is_empty() {
            return Ok(Self::new());
        }
        Ok(Self {
            matrix,
            radius,
            n_points_x: tx.nrows(),
            n_points_y: ty.nrows(),
        })
    }
}

fn device_matrix(tx: &Array2<f64>, ty: &Array2<f64>, radius: f64, theiler: usize) -> Result<Array2<bool>, GpuError> {
    if tx.nrows() == 0 || ty.nrows() == 0 || tx.ncols() == 0 || ty.ncols() == 0 || tx.ncols() != ty.ncols() {
        return Ok(Array2::default((0, 0)));
    }
    if !radius.is_finite() {
        return Err(GpuError::InvalidInput("radius must be finite"));
    }
    if bool_matrix_too_large(tx.nrows(), ty.nrows()) {
        return Err(GpuError::TooLarge {
            nrows: tx.nrows(),
            ncols: ty.nrows(),
            max: MAX_BOOL_MATRIX_CELLS,
        });
    }

    let left_points = row_major_points(tx);
    let right_owned = if std::ptr::eq(tx, ty) {
        None
    } else {
        Some(row_major_points(ty))
    };
    let right_slice: &[f64] = match &right_owned {
        Some(points) => points.as_ref(),
        None => left_points.as_ref(),
    };
    let mask = recurrence_mask(
        Trajectory {
            points: left_points.as_ref(),
            n: tx.nrows(),
            dim: tx.ncols(),
        },
        Trajectory {
            points: right_slice,
            n: ty.nrows(),
            dim: ty.ncols(),
        },
        radius,
        theiler,
    )?;
    Ok(expand(&mask))
}

fn row_major_points(traj: &Array2<f64>) -> Cow<'_, [f64]> {
    match traj.as_slice() {
        Some(slice) => Cow::Borrowed(slice),
        None => Cow::Owned(traj.iter().copied().collect()),
    }
}

fn expand(mask: &RecurrenceMask) -> Array2<bool> {
    let mut matrix = Array2::from_elem((mask.nrows, mask.ncols), false);
    for row in 0..mask.nrows {
        for col in 0..mask.ncols {
            if mask.bit(row, col) {
                matrix[[row, col]] = true;
            }
        }
    }
    matrix
}

#[cfg(test)]
mod tests {
    use ndarray::Array2;

    use super::*;

    #[cfg(not(feature = "hip"))]
    #[test]
    fn device_method_reports_unavailable_without_hip() {
        let traj = ndarray::array![[0.0, 0.0], [1.0, 0.0]];
        let err = RecurrencePlot::try_from_trajectory_on_device(&traj, 1.0, 0).unwrap_err();
        assert!(matches!(err, GpuError::Unavailable));
    }

    #[test]
    fn device_method_rejects_a_matrix_above_the_bool_cap() {
        let traj = Array2::<f64>::zeros((20_001, 1));
        let err = RecurrencePlot::try_from_trajectory_on_device(&traj, 1.0, 0).unwrap_err();
        assert!(matches!(err, GpuError::TooLarge { .. }));
    }

    #[test]
    fn non_finite_radius_is_rejected() {
        let traj = ndarray::array![[0.0, 1.0]];
        let err = RecurrencePlot::try_from_trajectory_on_device(&traj, f64::INFINITY, 0).unwrap_err();
        assert!(matches!(err, GpuError::InvalidInput("radius must be finite")));
    }

    #[test]
    fn empty_and_mismatched_inputs_match_the_cpu_constructors() {
        let empty = Array2::<f64>::zeros((0, 2));
        let plot = RecurrencePlot::try_from_trajectory_on_device(&empty, 1.0, 0).unwrap();
        assert_eq!(plot.n_points, 0);

        let row = ndarray::array![[0.0, 1.0]];
        let col = ndarray::array![[0.0], [1.0]];
        let cross = CrossRecurrencePlot::try_from_trajectories_on_device(&row, &col, 1.0, 0).unwrap();
        assert_eq!(cross.n_points_x, 0);
        assert_eq!(cross.n_points_y, 0);
    }
}
