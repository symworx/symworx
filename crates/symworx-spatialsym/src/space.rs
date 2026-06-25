// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Generic spatial dimensions and bounds for the playing area (sport-agnostic).

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
    /// Create new playing dimensions.
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
