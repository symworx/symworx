// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

#[cfg(feature = "supervision")]
use crate::error::BackendError;
use crate::server::Server;

/// Manages the lifecycle of backend processes.
pub struct ProcessManager {
    server: Server,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    /// Initiate a new process.
    pub fn new() -> Self {
        Self { server: Server::new() }
    }

    /// Start the process.
    pub fn start(&mut self) {
        self.server.start();
    }

    /// Stop the process.
    pub fn stop(&mut self) {
        self.server.stop();
    }

    #[cfg(feature = "supervision")]
    /// Placeholder for future supervised task registration.
    pub fn spawn_supervised<F>(&mut self, _name: &str, _task: F) -> Result<(), BackendError>
    where
        F: FnOnce() + Send + 'static,
    {
        // TODO: implement actual supervision
        Ok(())
    }
}
