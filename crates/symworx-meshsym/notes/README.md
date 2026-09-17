# Notes — MeshSym

Public surface: crate [README](../README.md). These files are the model.
They describe intended architecture, not shipped APIs.

MeshSym notes own the **graph and the layers** (L0–L4, typed edges, regime,
brief, quality gate). Measures and file bytes live in other crates. Arduino /
Pi Zero / Pi 5 are **roles**, not workspace members. See [design.md](design.md)
for the reuse table.

Do not add a `symview` Home key. Firmware stays out of the default build.

| File | Stays in MeshSym as | Points at / will move |
|:-----|:--------------------|:----------------------|
| [design.md](design.md) | Crate boundary, reuse, v1 | — |
| [architecture.md](architecture.md) | **Canonical L0–L4**; board roles | Out-of-tree firmware / node daemons |
| [analytics.md](analytics.md) | **Routing table only** (layer → crate, quality, emit/pull) | [Dynamics group-phase](../../symworx-dynamics/notes/group-phase-coherence.md); BioSym physiology; [SpatialSym XY phase](../../symworx-spatialsym/notes/phase-coherence.md); math / stats primitives |
| [contracts.md](contracts.md) | Domain objects (edge, regime, brief, quality) | On-disk encoding → `symworx-io` when we add files; wire shares the same objects |
| [glossary.md](glossary.md) | Locked MeshSym words | — |
| [related-work.md](related-work.md) | Product slice | — |
| [diagrams.md](diagrams.md) | L0–L4 / bus pictures | — |
| [lineage.md](lineage.md) | History | — |

Dated scratch, if any, should use `YYYY-MM-short-slug.md`. If a scratch file
disagrees with `design.md` or `architecture.md`, update the durable file on
purpose or discard the scratch.
