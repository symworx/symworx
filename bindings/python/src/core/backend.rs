// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

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
