// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Backend utilities for SymWorx.
//!
//! Shared process supervision, health snapshots, and a cloud-agnostic
//! object-store interface (local disk now; AWS S3 / Azure Blob later).
//!
//! Analysis I/O stays in `symworx-io`. This crate does not parse CSV,
//! FIT, or Parquet.

#![warn(missing_docs)]

/// Environment / cloud configuration.
pub mod config;
/// Error type.
pub mod error;
/// Liveness and readiness snapshots.
pub mod health;
/// Process and task table.
pub mod process_manager;
/// Server handle (bind + running flag).
pub mod server;
/// Object store (local now; AWS / Azure reserved).
pub mod store;

#[cfg(feature = "supervision")]
pub mod shutdown;

pub use config::{
    BackendConfig,
    CloudProvider,
};
pub use error::BackendError;
pub use health::{
    HealthReport,
    HealthState,
};
pub use process_manager::{
    ProcessManager,
    TaskInfo,
};
pub use server::Server;
pub use store::{
    LocalFsStore,
    ObjectStore,
    open_store,
};

/// Crate version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
