// symworx/crates/symworx-core/src/processing/traits.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use ndarray::ArrayView1;
use super::peaks::{Peak, PeakFinderBuilder};

pub trait PeakDetect {
    fn peaks(&self) -> PeakFinderBuilder;
}

impl<T> PeakDetect for T
where
    T: AsRef<[f64]>,
{
    fn peaks(&self) -> PeakFinderBuilder {
        PeakFinderBuilder::new(ArrayView1::from(self.as_ref()))
    }
}
