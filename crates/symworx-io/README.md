# SymWorx-IO

Canonical on-disk signal and activity I/O for the SymWorx workspace
(CSV, Parquet, IBI, FIT/activity, and related helpers).

This is a sub-crate of [`symworx-core`](../symworx-core/README.md). All other
crates must load and save signal files through this crate — do not depend on
parquet/polars stacks for on-disk I/O.

Also provides personal-archive path helpers (`VELOFIT_HOME`, activity discovery)
used by `symload` and the TUI.

Numeric tables (`load_numeric_table`): comma, tab, or whitespace; first-row
headers **or** explicit names. Kept columns are `f64` only (other columns
skipped). Example:

```rust
use symworx_io::{TableDelimiter, TableReadOptions, load_numeric_table};

let opts = TableReadOptions {
    delimiter: TableDelimiter::Whitespace,
    has_headers: false,
    names: Some(vec!["t_s".into(), "rr_s".into()]),
};
let table = load_numeric_table("series.txt", &opts)?;
let t = table.column("t_s");
```

## Citation

If you use this software, please cite it:

Berry, N. T. (2026). *SymWorx* [Computer software]. https://github.com/symworx/symworx

```bibtex
@software{Berry_SymWorx_2026,
  author  = {Berry, Nathaniel T.},
  license = {Apache-2.0},
  title   = {{SymWorx}},
  url     = {https://github.com/symworx/symworx},
  year    = {2026}
}
```

Add the version you used. [`CITATION.cff`](https://github.com/symworx/symworx/blob/main/CITATION.cff) and GitHub **Cite this repository** carry the current release.
