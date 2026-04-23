// core/src/backend/process_manager.rs
// Copyright (C) 2026 cSYMd, All rights reserved.

use crate::backend::server::Server;
use crate::backend::ProcessManager;


/// Manages the lifecycle of backend processes, including starting and stopping the server.
/// This struct provides a high-level interface for controlling the backend components of the application.
impl ProcessManager {
    pub fn new() -> Self {
        Self {
            server: Server::new(),
        }
    }

    pub fn start(&mut self) {
        self.server.start();
    }

    pub fn stop(&mut self) {
        self.server.stop();
    }
}
