# SymWorx-IO

Canonical on-disk signal and activity I/O for the SymWorx workspace
(CSV, Parquet, IBI, FIT/activity, and related helpers).

This is a sub-crate of [`symworx-core`](../symworx-core/README.md). All other
crates must load and save signal files through this crate — do not depend on
parquet/polars stacks for on-disk I/O.

Also provides personal-archive path helpers (`VELOFIT_HOME`, activity discovery)
used by `symload` and the TUI.

## Citation

If you use this software, please cite **SymWorx**, not this crate alone.
Name the crate in the paper if it helps; the bibliography title is still *SymWorx*.

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
