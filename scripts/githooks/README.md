# Git hooks (versioned)

These hooks live in the repo (not only under `.git/hooks/`) so every clone can
share the same local gates.

## Enable (once per clone)

From the repository root:

```bash
./scripts/githooks/install
```

Equivalent:

```bash
git config core.hooksPath scripts/githooks
chmod +x scripts/githooks/pre-commit
```

`core.hooksPath` is **local** to this clone (not committed). Teammates and new
clones need to run install once.

## What runs on commit

| Hook        | When                         | What |
|-------------|------------------------------|------|
| `pre-commit` | Staged `.rs` / `Cargo.toml` / `rustfmt.toml` / `Cargo.lock` | `cargo +nightly fmt --check` on the CI package list; also `symworx-tui` / `symworx-embed` if those paths are staged |

Clippy, tests, and Python bindings stay in CI — they are too slow/heavy
(OpenBLAS) for every local commit.

## Requirements

- `cargo` on `PATH`
- Nightly rustfmt (same as CI):

  ```bash
  rustup toolchain install nightly --component rustfmt
  ```

## Bypass

```bash
git commit --no-verify
```

Use sparingly; CI still enforces the same fmt gate.

## Disable

```bash
git config --unset core.hooksPath
```
