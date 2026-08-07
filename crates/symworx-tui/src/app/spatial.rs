// Copyright (c) 2026 PalEm Dynamics LLC
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
