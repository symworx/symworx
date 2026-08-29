# symworx-backend

Backend utilities for SymWorx: process table, health snapshots, and a
cloud-agnostic object store.

This crate does **not** own signal file formats. Load CSV / Parquet / FIT
through [`symworx-io`](../symworx-io/README.md).

## Features

- `supervision` — `graceful_shutdown()` (Ctrl+C / SIGTERM) via `tokio-util`.

Default build stays light: no AWS or Azure SDK.

## Status

| Piece | State |
|-------|--------|
| `BackendConfig` / `CloudProvider` | **Supported** (env: `SYMWORX_CLOUD=local\|aws\|azure`) |
| `ProcessManager` task table | **Supported** (register / start / stop / health) |
| `HealthReport` | **Supported** (no HTTP listener yet) |
| `LocalFsStore` | **Supported** |
| AWS S3 / Azure Blob I/O | **Reserved** — config validates; `open_store` returns `CloudDisabled` until an SDK feature exists |
| HTTP API | **Not started** |

## Cloud env

```bash
export SYMWORX_CLOUD=local          # or aws | azure
export SYMWORX_BIND=127.0.0.1:8080
export SYMWORX_OBJECT_ROOT=.symworx-backend
export SYMWORX_OBJECT_BUCKET=symworx-research   # required for aws/azure
export SYMWORX_OBJECT_PREFIX=csymd/sleep
export SYMWORX_REGION=us-east-1     # or AWS_REGION / AZURE_LOCATION
export SYMWORX_OBJECT_ENDPOINT=     # MinIO / Azurite / LocalStack
```

Do not put credentials in the repo. Use the platform identity
(IRSA / task role on ECS, managed identity on Azure App Service).

Design notes: [notes/cloud.md](notes/cloud.md).

## Usage

```toml
[dependencies]
symworx-backend = { version = "0.3.3" }
```

```rust
use symworx_backend::{ProcessManager, open_store, BackendConfig};

let cfg = BackendConfig::from_env().unwrap_or_default();
let mut pm = ProcessManager::with_config(cfg.clone());
pm.start();
pm.start_task("ingest").unwrap();
let _health = pm.health();

if cfg.provider.as_str() == "local" {
    let store = open_store(&cfg).unwrap();
    store.put("job/1.json", b"{}").unwrap();
}
```

## License

Licensed under the Apache License, Version 2.0.
