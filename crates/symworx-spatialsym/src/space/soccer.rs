// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Soccer / association-football IFAB Law 1 numbers (meters).
//!
//! Pitch **length and width** may vary inside the Law 1 ranges. Markings that
//! the Laws fix (goal, 6-yard box, 18-yard box, penalty mark, circles) are
//! constants and do **not** scale with pitch size.
//!
//! Yard colloquialisms appear only in docs: 18 yd ≈ 16.5 m, 6 yd ≈ 5.5 m,
//! 12 yd ≈ 11 m, 10 yd ≈ 9.15 m.

use crate::{
    error::{
        Result,
        SpatialError,
    },
    space::{
        PlayingDimensions,
        markings::{
            CenterCircle,
            EndBox,
            GoalSpec,
            PenaltyMark,
            PlayAreaMarkings,
        },
    },
};

/// Touchline (length) minimum, Law 1.
pub const LENGTH_MIN_M: f64 = 90.0;
/// Touchline (length) maximum, Law 1.
pub const LENGTH_MAX_M: f64 = 120.0;
/// Goal-line (width) minimum, Law 1.
pub const WIDTH_MIN_M: f64 = 45.0;
/// Goal-line (width) maximum, Law 1.
pub const WIDTH_MAX_M: f64 = 90.0;
/// FIFA stadium recommendation.
pub const LENGTH_DEFAULT_M: f64 = 105.0;
/// FIFA stadium recommendation.
pub const WIDTH_DEFAULT_M: f64 = 68.0;

/// Goal mouth width (8 yd).
pub const GOAL_WIDTH_M: f64 = 7.32;
/// Crossbar height (8 ft).
pub const GOAL_HEIGHT_M: f64 = 2.44;
/// Outer end-box depth (18 yd).
pub const OUTER_END_DEPTH_M: f64 = 16.5;
/// Inner end-box depth (6 yd).
pub const INNER_END_DEPTH_M: f64 = 5.5;
/// Penalty mark from the goal line (12 yd).
pub const PENALTY_FROM_GOAL_M: f64 = 11.0;
/// Center circle / penalty-arc radius (10 yd).
pub const CIRCLE_RADIUS_M: f64 = 9.15;
/// Corner arc.
pub const CORNER_ARC_M: f64 = 1.0;

/// Outer box width from posts: `goal + 2 × 16.5 m` (≈ 44 yd colloquial).
pub fn outer_end_width_m() -> f64 {
    GOAL_WIDTH_M + 2.0 * OUTER_END_DEPTH_M
}

/// Inner box width from posts: `goal + 2 × 5.5 m`.
pub fn inner_end_width_m() -> f64 {
    GOAL_WIDTH_M + 2.0 * INNER_END_DEPTH_M
}

/// IFAB-fixed markings (independent of pitch length/width).
pub fn ifab_markings() -> PlayAreaMarkings {
    PlayAreaMarkings {
        goal: GoalSpec {
            width_m: GOAL_WIDTH_M,
            height_m: GOAL_HEIGHT_M,
        },
        inner_end: EndBox {
            depth_m: INNER_END_DEPTH_M,
            width_m: inner_end_width_m(),
        },
        outer_end: EndBox {
            depth_m: OUTER_END_DEPTH_M,
            width_m: outer_end_width_m(),
        },
        penalty_mark: PenaltyMark {
            from_goal_line_m: PENALTY_FROM_GOAL_M,
        },
        penalty_arc_radius_m: CIRCLE_RADIUS_M,
        center_circle: CenterCircle {
            radius_m: CIRCLE_RADIUS_M,
        },
        corner_arc_radius_m: CORNER_ARC_M,
    }
}

/// Validate Law 1 ranges: length 90–120 m, width 45–90 m, length > width.
pub fn try_dimensions(length_m: f64, width_m: f64) -> Result<PlayingDimensions> {
    if !(LENGTH_MIN_M..=LENGTH_MAX_M).contains(&length_m) {
        return Err(SpatialError::InvalidParameter(format!(
            "soccer length must be {LENGTH_MIN_M}–{LENGTH_MAX_M} m, got {length_m}"
        )));
    }
    if !(WIDTH_MIN_M..=WIDTH_MAX_M).contains(&width_m) {
        return Err(SpatialError::InvalidParameter(format!(
            "soccer width must be {WIDTH_MIN_M}–{WIDTH_MAX_M} m, got {width_m}"
        )));
    }
    if length_m <= width_m {
        return Err(SpatialError::InvalidParameter(
            "soccer length must be greater than width".into(),
        ));
    }
    Ok(PlayingDimensions::new(length_m, width_m))
}

/// FIFA-recommended 105 × 68 m pitch plus IFAB markings.
pub fn default_pitch() -> (PlayingDimensions, PlayAreaMarkings) {
    (
        PlayingDimensions::new(LENGTH_DEFAULT_M, WIDTH_DEFAULT_M),
        ifab_markings(),
    )
}

/// Pitch of the given size (Law 1 ranges) plus fixed IFAB markings.
pub fn try_pitch(length_m: f64, width_m: f64) -> Result<(PlayingDimensions, PlayAreaMarkings)> {
    Ok((try_dimensions(length_m, width_m)?, ifab_markings()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_box_widths() {
        let (dims, m) = default_pitch();
        assert!((dims.length_m - 105.0).abs() < 1e-9);
        assert!((dims.width_m - 68.0).abs() < 1e-9);
        assert!((m.outer_end.width_m - (7.32 + 2.0 * 16.5)).abs() < 1e-9);
        assert!((m.inner_end.width_m - (7.32 + 2.0 * 5.5)).abs() < 1e-9);
        assert!((m.outer_end.depth_m - 16.5).abs() < 1e-9);
        assert!((m.inner_end.depth_m - 5.5).abs() < 1e-9);
        assert!((m.penalty_mark.from_goal_line_m - 11.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(try_dimensions(80.0, 68.0).is_err());
        assert!(try_dimensions(105.0, 40.0).is_err());
        assert!(try_dimensions(90.0, 90.0).is_err());
        assert!(try_pitch(105.0, 68.0).is_ok());
    }

    #[test]
    fn markings_do_not_scale_with_pitch() {
        let (_, a) = try_pitch(100.0, 64.0).unwrap();
        let (_, b) = try_pitch(110.0, 75.0).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn boxes_fit_law1_minimum_pitch() {
        let dims = try_dimensions(LENGTH_MIN_M, WIDTH_MIN_M).unwrap();
        let m = ifab_markings();
        assert!(m.outer_end.width_m < dims.width_m);
        assert!(2.0 * m.outer_end.depth_m < dims.length_m);
        assert!(m.inner_end.width_m < m.outer_end.width_m);
        assert!(m.goal.width_m < m.inner_end.width_m);
    }
}
