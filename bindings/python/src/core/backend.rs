// Copyright (c) 2026 SymWorx

use pyo3::prelude::*;

use symworx_core::backend::*;

// ===========================================================
// Backend 
// ===========================================================
// 
#[pyclass(name = "ProcessManager")]
pub struct PyProcessManager {
    pub process_manager: ProcessManager,
}
