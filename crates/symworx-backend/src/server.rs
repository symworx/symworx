// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use crate::config::BackendConfig;

/// Backend server handle (no HTTP listener yet).
///
/// Holds bind address and running flag so ECS / App Service wrappers can
/// share config with [`crate::ProcessManager`] before a real listener lands.
pub struct Server {
    config: BackendConfig,
    running: bool,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    /// Server with default local config.
    pub fn new() -> Self {
        Self::with_config(BackendConfig::default())
    }

    /// Server with explicit config.
    pub fn with_config(config: BackendConfig) -> Self {
        Self { config, running: false }
    }

    /// Config used at construction (env snapshot).
    pub fn config(&self) -> &BackendConfig {
        &self.config
    }

    /// Whether [`Self::start`] has been called without a matching stop.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Mark the server running.
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Mark the server stopped.
    pub fn stop(&mut self) {
        self.running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_stop_flag() {
        let mut s = Server::new();
        assert!(!s.is_running());
        s.start();
        assert!(s.is_running());
        s.stop();
        assert!(!s.is_running());
    }
}
