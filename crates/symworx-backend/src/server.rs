// core/src/backend/server.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use crate::backend::process_manager::ProcessManager;

/// Represents the server component of the backend, responsible for handling incoming requests and managing communication with clients.
pub struct Server {
    process_manager: ProcessManager,
}
