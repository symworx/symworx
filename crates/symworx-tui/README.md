# symworx-tui

**`symview`** — Terminal user interface for selecting, converting, and visualizing biosym / physiological signals in the Symworx ecosystem.

> **Status**: v0.1 (Early development)

## Features

- Interactive file browser for biosignal files
- Manual path input support
- File conversion: `.ibi` → CSV (via `symworx-io`) and `.parquet` → CSV (via Polars)
- Clean two-screen interface: **File Selection** ↔ **Visualization** (stub)
- Keyboard-driven (vim-friendly navigation)
- Designed for easy integration with the rest of the Symworx crates

## Quick Start

```bash
cargo run -p symworx-tui --bin symview
```
