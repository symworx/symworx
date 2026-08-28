// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Liveness / readiness snapshots for ECS, App Service, and local runs.

use crate::config::{BackendConfig, CloudProvider};

/// Coarse health for probes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    /// Process is up.
    Live,
    /// Process is up and ready to accept work.
    Ready,
    /// Up, but a dependency is missing or a task failed.
    Degraded,
}

impl HealthState {
    /// Wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Ready => "ready",
            Self::Degraded => "degraded",
        }
    }
}

/// Point-in-time backend health.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthReport {
    /// Aggregate state.
    pub state: HealthState,
    /// Crate version.
    pub version: String,
    /// Configured provider.
    pub provider: CloudProvider,
    /// Listen address (may be unused until an HTTP server exists).
    pub bind: String,
    /// Number of registered tasks currently marked running.
    pub tasks_running: usize,
    /// Number of registered tasks.
    pub tasks_total: usize,
    /// Human note (missing bucket, local root, etc.).
    pub detail: String,
}

impl HealthReport {
    /// Build a report from config and task counts.
    pub fn from_parts(
        cfg: &BackendConfig,
        tasks_running: usize,
        tasks_total: usize,
        ready: bool,
    ) -> Self {
        let state = if ready {
            HealthState::Ready
        } else if tasks_total == 0 {
            HealthState::Live
        } else {
            HealthState::Degraded
        };
        let detail = match cfg.provider {
            CloudProvider::Local => format!("fs:{}", cfg.object_root.display()),
            CloudProvider::Aws | CloudProvider::Azure => format!(
                "{} bucket={} prefix={}",
                cfg.provider.as_str(),
                cfg.bucket.as_deref().unwrap_or("-"),
                cfg.prefix
            ),
        };
        Self {
            state,
            version: crate::VERSION.to_string(),
            provider: cfg.provider,
            bind: cfg.bind.clone(),
            tasks_running,
            tasks_total,
            detail,
        }
    }

    /// True when probes should return 200.
    pub fn is_ready(&self) -> bool {
        matches!(self.state, HealthState::Ready | HealthState::Live)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_ready_report() {
        let cfg = BackendConfig::default();
        let h = HealthReport::from_parts(&cfg, 0, 0, true);
        assert_eq!(h.state, HealthState::Ready);
        assert!(h.is_ready());
        assert_eq!(h.provider, CloudProvider::Local);
    }
}
