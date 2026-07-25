// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! SpatialSym view enum.

/// Spatial sub-view (equivalent of sub-tabs inside the Spatial tab)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpatialView {
    #[default]
    Visualize,
    Generate,
    ImportData,
}
