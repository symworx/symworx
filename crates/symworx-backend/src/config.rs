// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Cloud-agnostic backend configuration.
//!
//! Reads a small set of environment variables that map onto local disk,
//! AWS S3, or Azure Blob without pulling in either vendor SDK.

use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use crate::error::BackendError;

/// Where object bytes and job state should live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    /// Filesystem under `object_root` (default; workstation / CI).
    Local,
    /// AWS S3 (or S3-compatible endpoint such as MinIO).
    Aws,
    /// Azure Blob Storage.
    Azure,
}

impl CloudProvider {
    /// Wire name used in env vars and health payloads.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Aws => "aws",
            Self::Azure => "azure",
        }
    }
}

impl FromStr for CloudProvider {
    type Err = BackendError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "local" | "fs" | "filesystem" => Ok(Self::Local),
            "aws" | "s3" | "amazon" => Ok(Self::Aws),
            "azure" | "az" | "blob" => Ok(Self::Azure),
            other => Err(BackendError::config(format!(
                "unknown SYMWORX_CLOUD={other:?}; expected local|aws|azure"
            ))),
        }
    }
}

/// Process / object-store settings for one backend instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendConfig {
    /// Target cloud (or local disk).
    pub provider: CloudProvider,
    /// Bind address for a future HTTP listener (`host:port`).
    pub bind: String,
    /// Local root when `provider == Local`.
    pub object_root: PathBuf,
    /// Bucket / container name when `provider` is AWS or Azure.
    pub bucket: Option<String>,
    /// Key prefix inside the bucket (research project / de-id partition).
    pub prefix: String,
    /// AWS region or Azure location.
    pub region: Option<String>,
    /// Optional custom endpoint (MinIO, Azurite, LocalStack).
    pub endpoint: Option<String>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            provider: CloudProvider::Local,
            bind: "127.0.0.1:8080".into(),
            object_root: PathBuf::from(".symworx-backend"),
            bucket: None,
            prefix: String::new(),
            region: None,
            endpoint: None,
        }
    }
}

impl BackendConfig {
    /// Load from process environment. Missing keys keep [`Default`].
    pub fn from_env() -> Result<Self, BackendError> {
        let mut cfg = Self::default();
        if let Ok(v) = env::var("SYMWORX_CLOUD") {
            cfg.provider = v.parse()?;
        }
        if let Ok(v) = env::var("SYMWORX_BIND") {
            if !v.is_empty() {
                cfg.bind = v;
            }
        }
        if let Ok(v) = env::var("SYMWORX_OBJECT_ROOT") {
            if !v.is_empty() {
                cfg.object_root = PathBuf::from(v);
            }
        }
        if let Ok(v) = env::var("SYMWORX_OBJECT_BUCKET") {
            if !v.is_empty() {
                cfg.bucket = Some(v);
            }
        }
        if let Ok(v) = env::var("SYMWORX_OBJECT_PREFIX") {
            cfg.prefix = v.trim_matches('/').to_string();
        }
        if let Ok(v) = env::var("SYMWORX_OBJECT_ENDPOINT") {
            if !v.is_empty() {
                cfg.endpoint = Some(v);
            }
        }
        cfg.region = first_env(&[
            "SYMWORX_REGION",
            "AWS_REGION",
            "AWS_DEFAULT_REGION",
            "AZURE_LOCATION",
        ]);
        cfg.validate()?;
        Ok(cfg)
    }

    /// Reject cloud providers that have no bucket name.
    pub fn validate(&self) -> Result<(), BackendError> {
        match self.provider {
            CloudProvider::Local => Ok(()),
            CloudProvider::Aws | CloudProvider::Azure => {
                if self.bucket.as_ref().map(|b| b.is_empty()).unwrap_or(true) {
                    Err(BackendError::config(format!(
                        "{} requires SYMWORX_OBJECT_BUCKET",
                        self.provider.as_str()
                    )))
                } else {
                    Ok(())
                }
            }
        }
    }
}

fn first_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| {
        env::var(k)
            .ok()
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_providers() {
        assert_eq!("aws".parse::<CloudProvider>().unwrap(), CloudProvider::Aws);
        assert_eq!("S3".parse::<CloudProvider>().unwrap(), CloudProvider::Aws);
        assert_eq!("azure".parse::<CloudProvider>().unwrap(), CloudProvider::Azure);
        assert_eq!("local".parse::<CloudProvider>().unwrap(), CloudProvider::Local);
        assert!("gcp".parse::<CloudProvider>().is_err());
    }

    #[test]
    fn cloud_requires_bucket() {
        let cfg = BackendConfig {
            provider: CloudProvider::Aws,
            bucket: None,
            ..BackendConfig::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn local_default_ok() {
        assert!(BackendConfig::default().validate().is_ok());
    }
}
