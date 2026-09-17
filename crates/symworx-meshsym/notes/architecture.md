# Architecture

Intended hierarchy. Nothing here is implemented in this crate yet.

```text
L0  wearable / MCU / strap
        │  primary pair (1 person ↔ 1 personal node)
        │  many channels per person are allowed
        ▼
personal node
        • body graph: L1 snapshots, quality, last briefs
        • RSSI table when peers exist (who I hear)
        │  features + RSSI + quality   (raw on pull only)
        ▼
unit node   (formation only; one box per formation, ~2–12 people)
        • L2 longer individual dynamics
        • L3 pairwise (physio, movement, proximity)
        • L4 regime + change detection
        • briefs back down
```

One person, one personal node. That node may fan in several L0 sensors.
A second unit node appears only when two formations must couple.

On a **body graph**, the personal node *is* the unit node of that person’s
sensors: L3/L4 run locally, and the brief is a cue to the same `sid`. A
formation unit node, if present, sees snapshots — not the haptic stream.

## Layers

| Layer | Where | Object | Timescale |
|:------|:------|:-------|:----------|
| L0 | MCU / strap | samples (IMU, RR, …) + quality bits; lightweight complexity proxies | samples |
| L1 | personal node | sensor-now / person-now: quality, cadence, short HRV, short-window entropy | seconds; **causal** |
| L2 | personal → unit | person-pattern: RQA / entropy on that series | tens of seconds–minutes |
| L3 | unit node (or personal node on a body graph) | pair: physio CRQA, movement CRQA/sync, RSSI class, relative phase, effort vs directional phase | windows |
| L4 | same as L3 | regime: network summaries + MdRQA/joint RQA + entropy of descriptors + cluster-phase | windows |

Cross-layer CRQA (person vs unit, movement vs proximity, operator vs agent
events) is L3/L4, not a sixth box.

Immediate feedback (haptic / visual later) sits on **L0/L1**. Formation L3/L4
windows are too slow to drive a cue. See [analytics.md](analytics.md).

## What goes up / down

**Up (personal → unit):** individual feature snapshot, IMU summary, RSSI
table, quality. Not 200 Hz IMU unless pulled.

**Down (unit → personal, or personal → self):** graph state, this node’s
edges, neighbor (or sibling-sensor) summaries, QA flags. Not everyone’s
waveforms.

The personal node’s **relative status** is: me, neighbors or sibling sensors
(id, proximity or placement class, last heard, last brief), last unit brief,
age of each field.

Higher layers may request more detailed data (longer windows, rawer series,
extended diagnostics). The *capability* is required; the policy is out of
scope here.

## Degraded mode

If the unit node is silent, the personal node still has L1, who it can hear
(or which local channels are alive), and the last brief, marked stale.
Gossip of last-known briefs is a later option, not v0.

If one L0 channel dies on a body graph, remaining edges stay useful; the
missing edge is `unknown`, not a tactical split.

## Clocks

Pairwise phase and IMU sync need a shared clock. The unit node is the
intended time lord on a formation graph; the personal node on a body graph.
If `max_clock_error` exceeds the event width, do not emit a phase score.

## RSSI

Use as reciprocal, filtered, **ordinal** proximity and as a time series (a
third edge type). Do not convert to meters in v0. An upgrade path (GPS, UWB)
can replace the spatial metric without changing L3/L4 questions.

Intra-person placement is anatomy / laterality, not RSSI. Do not invent a
radio edge between a belt and a watch.

## SpatialSym

Post-hoc XY (meters, bidirectional windows, pitch, `SpaceAction`) stays in
`symworx-spatialsym`. MeshSym does not grow a play-area API. Causal IMU
effort-phase is **applied** here (quality-gated edge); do not fork SpatialSym’s
XY pairwise API. Directional phase from GPS heading stays SpatialSym.

## Board roles (not crate dependencies)

Three roles, one portable library. Board trees are not workspace members and
must not appear in MeshSym’s `Cargo.toml`.

| Role | Typical box | Runs | Depends on MeshSym? |
|:-----|:------------|:-----|:--------------------|
| L0 strap / MCU | Arduino-class | Samples + quality bits; optional tiny complexity proxy | **No.** Bytes/contracts only. |
| Personal node | Pi Zero-class | Body-graph L1 (L3/L4 when this person *is* the unit) | **May** (library). No Arduino HAL in that binary’s default path. |
| Unit node | Pi 5-class | Formation L2–L4, briefs down | **May** (same library, different binary/role). |

Dependency direction: node daemons depend **on** MeshSym; MeshSym never
depends on firmware, GPIO, BLE, or a Pi image. `cargo test` for this crate
must not need a board. A Zero and a Pi 5 must not fork regime labels or
edge types — adapters only (serial path, radio, clock source).

When board code exists, keep it out of `crates/symworx-meshsym/` (sibling
repo or a non-member `firmware/` tree). `symworx-embed` remains the host
PPG serial/simulator path; a Pi node is not embed.

## Field path (later)

Pairing, AEAD transport, encrypted volumes, and signed firmware are not
this crate. Default ingest remains snapshots and quality, not waveforms.
Recordings are treated as human-subjects data even in an open repository:
synthetic fixtures in git; real extracts outside the tree.

## First proofs

Do not ship two hardware proofs at once.

- Analysis: synthetic L3, two or three nodes, known cartoons (body channels
  or people).
- Field (later): three people, known split/together, after contracts exist.

## See also

- [design.md](design.md)
- [analytics.md](analytics.md)
- [contracts.md](contracts.md)
- [diagrams.md](diagrams.md)
