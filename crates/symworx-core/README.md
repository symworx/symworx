# SymWorx-Core

Convenience re-export crate for the SymWorx workspace: math, stats, signal,
dynamics, and I/O behind a single dependency.

Most application code can depend on `symworx-core` instead of listing every
leaf crate. Feature flags (e.g. stats `linalg`) follow the workspace defaults
documented in the root [README](../../README.md) and [AGENTS.md](../../AGENTS.md).

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
