// Copyright (C) 2026 cSYMd, All rights reserved.

use ndarray::ArrayView1;

use super::peaks::{PeakFinder, PeakFinderBuilder};

pub trait PeakDetect {
    fn peaks(&self) -> PeakFinderBuilder<'_>;
}

impl<T> PeakDetect for T
where
    T: AsRef<[f64]>,
{
    fn peaks(&self) -> PeakFinderBuilder<'_> {
        PeakFinder::new(ArrayView1::from(self.as_ref()))
    }
}
