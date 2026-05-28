# symworx-backend

Backend utilities for SymWorx.

This crate provides shared infrastructure and process supervision tools intended for use by higher-level crates in the SymWorx workspace.

## Features

- `supervision` — Enables process supervision utilities, error types, and graceful shutdown helpers. Pulls in `thiserror` and `tokio-util`.

## Current Status

This crate is in early development. The `supervision` feature currently provides:

- `BackendError`
- `graceful_shutdown()` helper
- Basic `ProcessManager` with a placeholder supervised task API

## Usage

```toml
[dependencies]
symworx-backend = { version = "...", features = ["supervision"] }
```

## License

Licensed under the Mozilla Public License, Version 2.0.