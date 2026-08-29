// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

use std::collections::BTreeMap;

use crate::{
    config::BackendConfig,
    error::BackendError,
    health::HealthReport,
    server::Server,
};

/// Snapshot of one named task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInfo {
    /// Task name (stable id).
    pub name: String,
    /// Whether the manager considers it running.
    pub running: bool,
}

/// Manages the lifecycle of backend processes and named tasks.
pub struct ProcessManager {
    server: Server,
    tasks: BTreeMap<String, TaskInfo>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    /// Create a manager with default local config.
    pub fn new() -> Self {
        Self::with_config(BackendConfig::default())
    }

    /// Create a manager bound to `config`.
    pub fn with_config(config: BackendConfig) -> Self {
        Self {
            server: Server::with_config(config),
            tasks: BTreeMap::new(),
        }
    }

    /// Borrow the server handle.
    pub fn server(&self) -> &Server {
        &self.server
    }

    /// Mutable server handle.
    pub fn server_mut(&mut self) -> &mut Server {
        &mut self.server
    }

    /// Start the server flag.
    pub fn start(&mut self) {
        self.server.start();
    }

    /// Stop the server flag and mark all tasks stopped.
    pub fn stop(&mut self) {
        self.server.stop();
        for task in self.tasks.values_mut() {
            task.running = false;
        }
    }

    /// Register a named task (idempotent). Does not start it.
    pub fn register(&mut self, name: &str) -> Result<(), BackendError> {
        let name = require_name(name)?;
        self.tasks
            .entry(name.clone())
            .or_insert(TaskInfo { name, running: false });
        Ok(())
    }

    /// Mark `name` running. Registers first if needed.
    pub fn start_task(&mut self, name: &str) -> Result<(), BackendError> {
        let name = require_name(name)?;
        let task = self
            .tasks
            .entry(name.clone())
            .or_insert(TaskInfo { name, running: false });
        task.running = true;
        if !self.server.is_running() {
            self.server.start();
        }
        Ok(())
    }

    /// Mark `name` stopped.
    pub fn stop_task(&mut self, name: &str) -> Result<(), BackendError> {
        let name = require_name(name)?;
        match self.tasks.get_mut(&name) {
            Some(task) => {
                task.running = false;
                Ok(())
            }
            None => Err(BackendError::task(format!("unknown task {name}"))),
        }
    }

    /// Registered tasks in name order.
    pub fn list(&self) -> Vec<TaskInfo> {
        self.tasks.values().cloned().collect()
    }

    /// How many tasks are marked running.
    pub fn running_count(&self) -> usize {
        self.tasks.values().filter(|t| t.running).count()
    }

    /// Health snapshot for probes.
    pub fn health(&self) -> HealthReport {
        let running = self.running_count();
        let total = self.tasks.len();
        let ready = self.server.is_running();
        HealthReport::from_parts(self.server.config(), running, total, ready)
    }

    /// Register a future supervised task. Records the name only; the
    /// closure is not scheduled until a runtime supervisor exists.
    #[cfg(feature = "supervision")]
    pub fn spawn_supervised<F>(&mut self, name: &str, _task: F) -> Result<(), BackendError>
    where
        F: FnOnce() + Send + 'static,
    {
        self.register(name)?;
        self.start_task(name)
    }
}

fn require_name(name: &str) -> Result<String, BackendError> {
    let name = name.trim();
    if name.is_empty() {
        Err(BackendError::task("task name is empty"))
    } else {
        Ok(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_start_stop() {
        let mut pm = ProcessManager::new();
        pm.register("ingest").unwrap();
        assert_eq!(pm.list().len(), 1);
        assert_eq!(pm.running_count(), 0);
        pm.start_task("ingest").unwrap();
        assert_eq!(pm.running_count(), 1);
        assert!(pm.server().is_running());
        pm.stop_task("ingest").unwrap();
        assert_eq!(pm.running_count(), 0);
        pm.stop();
        assert!(!pm.server().is_running());
    }

    #[test]
    fn stop_unknown_fails() {
        let mut pm = ProcessManager::new();
        assert!(pm.stop_task("nope").is_err());
    }
}
