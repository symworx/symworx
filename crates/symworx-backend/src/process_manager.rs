// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

use crate::server::Server;

/// Manages the lifecycle of backend processes.
pub struct ProcessManager {
    server: Server,
}

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
