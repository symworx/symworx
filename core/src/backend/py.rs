// core/src/backend/py.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use pyo3::prelude::*;

// ===========================================================
// Backend 
// ===========================================================
// 
// -----------------------------------------------------------
#[pyclass(name = "ProcessManager")]
pub struct PyProcessManager {
    pub process_manager: ProcessManager,
}
