# SymWorx-IO

Canonical on-disk signal and activity I/O for the SymWorx workspace
(CSV, Parquet, IBI, FIT/activity, and related helpers).

This is a sub-crate of [`symworx-core`](../symworx-core/README.md). All other
crates must load and save signal files through this crate — do not depend on
parquet/polars stacks for on-disk I/O.

Also provides personal-archive path helpers (`VELOFIT_HOME`, activity discovery)
used by `symload` and the TUI.
