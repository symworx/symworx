// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Live IMU / RSSI coupling analysis (L0–L4).
//!
//! MeshSym treats **relations** as the object: quality-gated typed edges
//! (physiology, movement, radio proximity) on a body graph (many sensors,
//! one subject) or a formation graph (many people). Live scores are **causal**
//! (no future window).
//!
//! This crate is a documentation-first scaffold. No analysis API is shipped
//! yet. Post-hoc XY / GPS trajectory work stays in `symworx-spatialsym`.
//! RQA, cluster-phase, and entropy stay in `symworx-dynamics`. Physiology
//! primitives stay in `symworx-biosym`. The PPG JSON-line host path stays in
//! `symworx-embed`.
//!
//! Design notes live in `notes/` next to this crate.

#![doc(html_root_url = "https://docs.rs/symworx-meshsym")]

/// Current version of the `symworx-meshsym` crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_nonempty() {
        assert!(!super::VERSION.is_empty());
    }
}
