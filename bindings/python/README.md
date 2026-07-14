# SymWorx Python bindings

Unified package **`symworx`**: PyO3 wrappers over the Rust crates (`biosym`, `loadsym`, `core`).

## Layout

| Path | Role |
|------|------|
| `src/` | Rust extension (`symworx._lib`) — PyO3 classes and functions |
| `symworx/` | Pure-Python package that re-exports from `_lib` |
| `tests/` | `pytest` against the installed package |
| `examples/` | Small demos |
| `pyproject-*.toml` | Optional **split** packages (`symworx_biosym`, …); secondary to the unified build |

Users import the public surface:

```python
from symworx import biosym, loadsym, core
```

The private extension is `symworx._lib` (attribute access from package code only).

## Develop & test

From the **workspace root**:

```bash
python -m venv .venv && .venv/bin/pip install "maturin>=1.5,<2.0" pytest
VIRTUAL_ENV=$PWD/.venv .venv/bin/maturin develop --manifest-path bindings/python/Cargo.toml
.venv/bin/pytest bindings/python/tests/ -q --tb=line
```

Optional IMAP helpers need OpenSSL and:

```bash
maturin develop --manifest-path bindings/python/Cargo.toml --features email
```
