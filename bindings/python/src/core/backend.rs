// symworx/bindings/python/src/core/backend.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

use symworx_core::backend::{

// ===========================================================
// Backend 
// ===========================================================
// 
// -----------------------------------------------------------
#[pyclass(name = "ProcessManager")]
pub struct PyProcessManager {
    pub process_manager: ProcessManager,
}
