// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use super::peaks::PeakFinderBuilder;

/// Provides a `.peaks()` method for slices of `f64`.
pub trait PeakDetect {
    /// Returns builder to configur epeak detection.
    fn peaks(&self) -> PeakFinderBuilder<'_>;
}

impl<T> PeakDetect for T
where
    T: AsRef<[f64]>,
{
    fn peaks(&self) -> PeakFinderBuilder<'_> {
        PeakFinderBuilder::from_slice(self.as_ref())
    }
}
