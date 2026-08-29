// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Object-store abstraction.
//!
//! Local disk is implemented. AWS S3 and Azure Blob share the same key
//! space (`prefix/key`) so a later `aws` / `azure` feature can drop in
//! without changing callers. Signal file formats still go through
//! `symworx-io`; this store is for opaque backend blobs (models, job
//! artifacts, de-identified exports).

use std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
};

use crate::{
    config::{
        BackendConfig,
        CloudProvider,
    },
    error::BackendError,
};

/// Put / get / list opaque objects by key.
pub trait ObjectStore: Send + Sync {
    /// Write `bytes` at `key` (relative, no leading slash).
    fn put(&self, key: &str, bytes: &[u8]) -> Result<(), BackendError>;
    /// Read an object.
    fn get(&self, key: &str) -> Result<Vec<u8>, BackendError>;
    /// Whether the key exists.
    fn exists(&self, key: &str) -> Result<bool, BackendError>;
    /// Delete if present.
    fn delete(&self, key: &str) -> Result<(), BackendError>;
    /// Keys under `prefix` (non-recursive listing is acceptable).
    fn list_prefix(&self, prefix: &str) -> Result<Vec<String>, BackendError>;
}

/// Filesystem object store.
#[derive(Debug, Clone)]
pub struct LocalFsStore {
    root: PathBuf,
    prefix: String,
}

impl LocalFsStore {
    /// Create (and mkdir) a store under `root`.
    pub fn open(root: impl AsRef<Path>, prefix: impl Into<String>) -> Result<Self, BackendError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|e| BackendError::store(e.to_string()))?;
        Ok(Self {
            root,
            prefix: trim_key(&prefix.into()).to_string(),
        })
    }

    fn resolve(&self, key: &str) -> Result<PathBuf, BackendError> {
        let key = sanitize_key(key)?;
        let mut rel = PathBuf::new();
        if !self.prefix.is_empty() {
            rel.push(&self.prefix);
        }
        rel.push(key);
        Ok(self.root.join(rel))
    }
}

impl ObjectStore for LocalFsStore {
    fn put(&self, key: &str, bytes: &[u8]) -> Result<(), BackendError> {
        let path = self.resolve(key)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| BackendError::store(e.to_string()))?;
        }
        fs::write(path, bytes).map_err(|e| BackendError::store(e.to_string()))
    }

    fn get(&self, key: &str) -> Result<Vec<u8>, BackendError> {
        let path = self.resolve(key)?;
        fs::read(path).map_err(|e| BackendError::store(e.to_string()))
    }

    fn exists(&self, key: &str) -> Result<bool, BackendError> {
        Ok(self.resolve(key)?.is_file())
    }

    fn delete(&self, key: &str) -> Result<(), BackendError> {
        let path = self.resolve(key)?;
        if path.is_file() {
            fs::remove_file(path).map_err(|e| BackendError::store(e.to_string()))?;
        }
        Ok(())
    }

    fn list_prefix(&self, prefix: &str) -> Result<Vec<String>, BackendError> {
        let path = self.resolve(prefix).unwrap_or_else(|_| self.root.clone());
        let dir = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(&self.root).to_path_buf()
        };
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for ent in fs::read_dir(&dir).map_err(|e| BackendError::store(e.to_string()))? {
            let ent = ent.map_err(|e| BackendError::store(e.to_string()))?;
            if ent.path().is_file() {
                if let Some(name) = ent.file_name().to_str() {
                    out.push(name.to_string());
                }
            }
        }
        out.sort();
        Ok(out)
    }
}

/// Open the store described by `cfg`.
///
/// Local always works. `aws` / `azure` return [`BackendError::CloudDisabled`]
/// until an SDK feature is added.
pub fn open_store(cfg: &BackendConfig) -> Result<Box<dyn ObjectStore>, BackendError> {
    cfg.validate()?;
    match cfg.provider {
        CloudProvider::Local => {
            let store = LocalFsStore::open(&cfg.object_root, &cfg.prefix)?;
            Ok(Box::new(store))
        }
        CloudProvider::Aws => Err(BackendError::CloudDisabled {
            provider: "aws".into(),
            hint: "use SYMWORX_CLOUD=local or add an `aws` feature with the S3 SDK".into(),
        }),
        CloudProvider::Azure => Err(BackendError::CloudDisabled {
            provider: "azure".into(),
            hint: "use SYMWORX_CLOUD=local or add an `azure` feature with the Blob SDK".into(),
        }),
    }
}

fn trim_key(key: &str) -> &str {
    key.trim().trim_matches('/')
}

fn sanitize_key(key: &str) -> Result<&str, BackendError> {
    let key = trim_key(key);
    if key.is_empty() {
        return Err(BackendError::store("empty object key"));
    }
    if key.split('/').any(|p| p == ".." || p == ".") {
        return Err(BackendError::store("object key must not contain '.' or '..'"));
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BackendConfig;

    #[test]
    fn local_put_get_list_delete() {
        let dir = std::env::temp_dir().join(format!("symworx-backend-store-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let store = LocalFsStore::open(&dir, "run1").unwrap();
        store.put("models/a.bin", b"abc").unwrap();
        assert!(store.exists("models/a.bin").unwrap());
        assert_eq!(store.get("models/a.bin").unwrap(), b"abc");
        let listed = store.list_prefix("models").unwrap();
        assert!(listed.iter().any(|k| k == "a.bin"));
        store.delete("models/a.bin").unwrap();
        assert!(!store.exists("models/a.bin").unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_dotdot() {
        let dir = std::env::temp_dir().join(format!("symworx-backend-store-dd-{}", std::process::id()));
        let store = LocalFsStore::open(&dir, "").unwrap();
        assert!(store.put("../x", b"no").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn aws_open_is_disabled() {
        let cfg = BackendConfig {
            provider: CloudProvider::Aws,
            bucket: Some("symworx-research".into()),
            ..BackendConfig::default()
        };
        match open_store(&cfg) {
            Err(BackendError::CloudDisabled { provider, .. }) => assert_eq!(provider, "aws"),
            Err(other) => panic!("expected CloudDisabled, got {other}"),
            Ok(_) => panic!("expected CloudDisabled, store opened"),
        }
    }
}
