// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Sport-agnostic play-area markings (meters).
//!
//! Sport-specific numbers (soccer IFAB, later others) live in sibling modules
//! such as [`super::soccer`]. This file only defines geometry: goal mouth,
//! inner/outer end boxes, penalty mark, center circle.

use crate::{
    geometry::Point2,
    space::PlayingDimensions,
};

/// Goal mouth on the goal line (meters).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GoalSpec {
    /// Width of the goal mouth along the goal line.
    pub width_m: f64,
    /// Crossbar height (unused in 2-D plan view; stored for completeness).
    pub height_m: f64,
}

/// Axis-aligned rectangle off a goal line, centered on the long axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EndBox {
    /// How far the box extends into the playing area from the goal line.
    pub depth_m: f64,
    /// Full width of the box (parallel to the goal line).
    pub width_m: f64,
}

/// Spot on the long axis, measured from the goal line into the playing area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PenaltyMark {
    /// Distance from the goal line (meters).
    pub from_goal_line_m: f64,
}

/// Circle at the origin (center of a centered [`PlayingDimensions`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CenterCircle {
    /// Radius (meters).
    pub radius_m: f64,
}

/// Full set of end-zone / midfield markings for a rectangular play area.
///
/// Length and width of the area come from [`PlayingDimensions`]; these values
/// do **not** scale when the outer rectangle changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayAreaMarkings {
    /// Goal mouth.
    pub goal: GoalSpec,
    /// Inner end box (soccer “6-yard” / goal area).
    pub inner_end: EndBox,
    /// Outer end box (soccer “18-yard” / penalty area).
    pub outer_end: EndBox,
    /// Penalty / set-piece mark.
    pub penalty_mark: PenaltyMark,
    /// Arc radius about the penalty mark (meters).
    pub penalty_arc_radius_m: f64,
    /// Center circle.
    pub center_circle: CenterCircle,
    /// Corner-arc radius (meters).
    pub corner_arc_radius_m: f64,
}

/// World-axis-aligned rectangle: `(x_min, y_min, width, height)`.
pub type WorldRect = (f64, f64, f64, f64);

impl PlayAreaMarkings {
    /// Outer/inner end box at the +x (`true`) or −x (`false`) goal line.
    pub fn end_box_rect(&self, dims: PlayingDimensions, plus_x: bool, inner: bool) -> WorldRect {
        let b = if inner { self.inner_end } else { self.outer_end };
        let (xmin, xmax, _, _) = dims.bounds();
        let half_w = b.width_m / 2.0;
        if plus_x {
            (xmax - b.depth_m, -half_w, b.depth_m, b.width_m)
        } else {
            (xmin, -half_w, b.depth_m, b.width_m)
        }
    }

    /// Penalty mark at the +x or −x end, on the long axis (`y = 0`).
    pub fn penalty_spot(&self, dims: PlayingDimensions, plus_x: bool) -> Point2 {
        let (xmin, xmax, _, _) = dims.bounds();
        let x = if plus_x {
            xmax - self.penalty_mark.from_goal_line_m
        } else {
            xmin + self.penalty_mark.from_goal_line_m
        };
        Point2::new(x, 0.0)
    }

    /// Goal-line segment endpoints (world), +x or −x end.
    pub fn goal_segment(&self, dims: PlayingDimensions, plus_x: bool) -> (Point2, Point2) {
        let (xmin, xmax, _, _) = dims.bounds();
        let x = if plus_x { xmax } else { xmin };
        let h = self.goal.width_m / 2.0;
        (Point2::new(x, -h), Point2::new(x, h))
    }
}
