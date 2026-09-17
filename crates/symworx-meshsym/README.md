# symworx-meshsym

Live coupling analysis on a **quality-gated graph**: IMU and radio proximity
(RSSI) as typed edges, with physiology as a third edge when those primitives
already exist elsewhere in the workspace.

Two instantiations share the same objects (node, edge, snapshot, brief, regime):

- **Body graph** — many L0 channels on one `sid` (control / feedback).
- **Formation graph** — one personal node per person (unit coupling).

Analyses on the live path are **causal**. This is not a GPS / pitch crate.

## Current status

| Piece | State |
|:------|:------|
| Design notes (`notes/`) | **Supported** (architecture, analytics, contracts, glossary) |
| Types / snapshot / brief | **Not started** |
| Synthetic L3 (known cartoons) | **Not started** |
| Live IMU / RSSI ingest | **Not started** |
| Firmware / node daemons | **Out of tree** (same rule as `symworx-embed`) |
| TUI workflow | **Not started** (do not add a Home key yet) |

Default build is `thiserror` only. Nothing else in the workspace, including the
TUI, links this crate yet.

## Why a separate crate

[`symworx-spatialsym`](../symworx-spatialsym/README.md) is post-hoc **XY**
(GPS, LPS, or video) in meters: trajectories, path linearity, pitch markings,
`SpaceAction`. Directional phase that needs a heading from position stays there.

MeshSym is the live **relation** path: IMU envelopes, ordinal RSSI (not meters),
quality → `unknown`, L0–L4 summaries, and a regime label. It **calls** dynamics
(CRQA, cluster-phase, entropy) and BioSym (HRV, gait, respiration); it does not
reimplement them.

`symworx-embed` remains the PPG JSON-line host stream. Do not grow it into a
mesh.

## What it is not

- A team GPS / LPS tracker (Catapult, Kinexon) — that family is SpatialSym when
  you have positions.
- A single “sync” or “group coherence” score.
- Diagnosis, readiness-for-duty, or a consumer coach.
- Firmware, pairing, or a phone app.

## Design notes

Public surface is this README. The model lives next to the crate:

| Note | Contents |
|:-----|:---------|
| [notes/design.md](notes/design.md) | Crate boundary, reuse, locked choices, v1 slice |
| [notes/architecture.md](notes/architecture.md) | L0–L4, body vs formation, up/down, degraded picture |
| [notes/analytics.md](notes/analytics.md) | Layer → crate routing (quality, emit/pull); measures stay elsewhere |
| [notes/contracts.md](notes/contracts.md) | Domain objects; disk → io later; wire shares the same shapes |
| [notes/glossary.md](notes/glossary.md) | Locked vocabulary |
| [notes/related-work.md](notes/related-work.md) | Cousins and the slice this crate is aimed at |
| [notes/diagrams.md](notes/diagrams.md) | Layer and contract diagrams |
| [notes/lineage.md](notes/lineage.md) | Earlier private field-kit notes (SymMesh) rewritten here |

If a dated note disagrees with `design.md` / `architecture.md`, update the
durable file on purpose.

## Usage

```rust
use symworx_meshsym::VERSION;

assert!(!VERSION.is_empty());
```

```bash
cargo test --lib -p symworx-meshsym
```

## Data

Synthetic fixtures only in git (when they exist). Do not commit recordings of
real people, identifiers, or device keys. Signal bytes still go through
[`symworx-io`](../symworx-io/README.md).

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
