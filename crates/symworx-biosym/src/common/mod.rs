// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Common shared primitives across SymWorx BioSym domains
//! (physiology, biomechanics, etc.).
//!
//! This is the preferred home for cross-cutting utilities such as
//! `IntervalSeries` (for RR, breath, stride, and other event intervals) and
//! processing parameters (`common::processing`) so that future biomechanics
//! efforts (CMJ, pedaling, etc.) and physiology can both rely on them.

pub mod intervals;
pub mod processing;

pub use intervals::IntervalSeries;
