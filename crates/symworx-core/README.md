# SymWorx-Core

Convenience re-export crate for the SymWorx workspace: math, stats, signal,
dynamics, and I/O behind a single dependency.

Most application code can depend on `symworx-core` instead of listing every
leaf crate. Feature flags (e.g. stats `linalg`) follow the workspace defaults
documented in the root [README](../../README.md) and [AGENTS.md](../../AGENTS.md).
