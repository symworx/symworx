# Notes — MeshSym crate

Public surface: crate README. This file is the model and reuse notes.

## Why this crate

SpatialSym answers *where agents went on a plane* (post-hoc XY). The rest of
SymWorx already has physiology primitives, RQA/CRQA, cluster-phase, and a PPG
host stream. What it lacks is a place for **live coupling**: typed edges with
quality gates, a regime of that graph, and a compact brief back down.

MeshSym is that place. Nodes may be sensors on one person (body graph) or
personal nodes in a small formation (formation graph). The objects are the
same; the edge vocabulary is not.

## Reuse (this workspace)

| Need | Existing piece |
|:-----|:---------------|
| Successive differences / tiny L0 proxies | `symworx-math::series` |
| Circular stats on relative phase | `symworx-math::circular` |
| RMSSD / SDNN / basic descriptors | `symworx-stats` |
| Cluster-phase, relative phase, Kuramoto *R* | `symworx-dynamics::phase` |
| RQA / CRQA, entropy | `symworx-dynamics` (MdRQA / CRQA networks still notes) |
| Short HRV, PPG, respiration, gait, later RSA | `symworx-biosym` |
| Filters, peaks, Kalman | `symworx-signal` |
| PID / LTI / SINDYc | `symworx-dynamics::control` (mesh feeds; does not reimplement) |
| Causal IMU effort-phase on a live edge | MeshSym **calls** a 1-D helper; SpatialSym keeps XY directional phase |
| PPG JSON-line host stream | `symworx-embed` (`sid`; firmware out of tree) |
| XY trajectories, pitch, `SpaceAction` | `symworx-spatialsym` |
| On-disk snapshot / brief / series files | `symworx-io` (when those files exist) |
| Arduino / Pi Zero / Pi 5 images | Out of tree — [architecture.md](architecture.md) board roles |
| Domain errors | `thiserror` enums — **not** `SymError` on the default path |
| Subject id | `sid` (same rule as embed: never emit `patient_id`) |

Do not take a database connection. Catalog work stays in `symworx-dbsym`.

## Locked choices

| Choice | Decision |
|:-------|:---------|
| Object | Quality-gated typed graph + regime + brief |
| Instantiations | Body graph (many L0 per `sid`) and formation graph (many people) |
| Live scores | Causal windows only |
| RSSI | Ordinal class (near / mid / fading / gone), not meters |
| XY / GPS | SpatialSym; MeshSym does not convert RSSI to position |
| Math | Call dynamics / BioSym / math / stats / signal; wrap; do not fork |
| Default deps | `thiserror` only |
| Firmware / radios / apps | Out of tree. Arduino talks contracts; Pi nodes may depend **on** this crate, never the reverse |
| TUI | No Home key until there is a consumer |
| Interface | Library first; CLI only when there is a command to run |
| Data in git | Synthetic fixtures only |

## v1 slice (when code starts)

1. Contracts: snapshot, quality, edge `{type, score | unknown}`, brief, regime.
   Many L0 ids per `sid`.
2. Synthetic L3: two or three nodes, known cartoons (together / intended
   split / unintended fragment), IMU-like series + ordinal proximity.
3. Causal effort-phase + quality gate (`unknown` on clock error / dropout).
4. A first L4 label from components + coupling so the object exists.
5. Tests and an example. No `symview` tab.

Hardware, RSSI honesty over the air, and briefs-down on a radio wait until
that slice is boring.

## Out of tree (on purpose)

Arduino-class L0, Pi Zero personal-node daemons, Pi 5 unit-node daemons,
pairing, transport, and phone / Flutter apps are not this crate and are
not workspace members. Portable graph logic stays here. When a field path
exists, **it** depends on MeshSym (except the MCU, which shares bytes only).

## See also

- [architecture.md](architecture.md) — layers
- [glossary.md](glossary.md) — words we use and do not use
- SpatialSym phase note: [`../../symworx-spatialsym/notes/phase-coherence.md`](../../symworx-spatialsym/notes/phase-coherence.md)
- Dynamics group-phase: [`../../symworx-dynamics/notes/group-phase-coherence.md`](../../symworx-dynamics/notes/group-phase-coherence.md)
