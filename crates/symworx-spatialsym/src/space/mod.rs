// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Play-area dimensions and markings.
//!
//! - [`PlayingDimensions`] — outer rectangle (meters), origin-centered.
//! - [`markings`] — sport-agnostic goal / end-box / circle geometry.
//! - [`soccer`] — IFAB Law 1 stock numbers (variable pitch, fixed boxes).
//!
//! Further sports should add `space/<sport>.rs` (or `space/<sport>/`) that
//! fill [`PlayAreaMarkings`] rather than introducing sport names in the
//! generic types.

/// Sport-agnostic end-zone and midfield markings.
pub mod markings;
/// Soccer IFAB Law 1 presets (meters).
pub mod soccer;

pub use markings::{
    CenterCircle,
    EndBox,
    GoalSpec,
    PenaltyMark,
    PlayAreaMarkings,
    WorldRect,
};

/// Rectangular playing area dimensions (in meters). Sport-agnostic metadata for field size.
///
/// (Renamed from the previous `ArenaSpec` to better reflect its role as field dimensions
/// alongside other metadata such as goal positions.)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayingDimensions {
    /// Length of the playing area along the primary (x) axis (meters).
    pub length_m: f64,
    /// Width of the playing area along the secondary (y) axis (meters).
    pub width_m: f64,
}

impl PlayingDimensions {
    /// Create new playing dimensions (no sport-specific range check).
    pub fn new(length_m: f64, width_m: f64) -> Self {
        Self { length_m, width_m }
    }

    /// Returns (xmin, xmax, ymin, ymax) assuming centered at origin (0,0).
    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        let hx = self.length_m / 2.0;
        let hy = self.width_m / 2.0;
        (-hx, hx, -hy, hy)
    }
}
