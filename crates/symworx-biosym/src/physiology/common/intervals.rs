// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Re-export of the crate-wide `IntervalSeries` (promoted to `common::intervals`
//! for clean separation between physiology and biomechanics domains).
//!
//! This file exists for backward compatibility with code that imports from
//! `physiology::common::intervals`. New code should prefer `common::IntervalSeries`
//! or `crate::common::IntervalSeries`.

pub use crate::common::IntervalSeries;
