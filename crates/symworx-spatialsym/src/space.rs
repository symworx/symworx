// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Generic spatial bounds, arenas, and free-space primitives (sport-agnostic).

/// Rectangular play area / arena specification (dimensions in meters).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArenaSpec {
    /// Length of the arena along the primary (x) axis (meters).
    pub length_m: f64,
    /// Width of the arena along the secondary (y) axis (meters).
    pub width_m: f64,
}

impl ArenaSpec {
    /// Create a new arena spec.
    pub fn new(length_m: f64, width_m: f64) -> Self {
        Self { length_m, width_m }
    }

    /// Returns (xmin, xmax, ymin, ymax) centered at origin.
    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        let hx = self.length_m / 2.0;
        let hy = self.width_m / 2.0;
        (-hx, hx, -hy, hy)
    }
}
