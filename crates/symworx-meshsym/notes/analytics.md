# Analytics — layer routing

Which **existing** crate fills each layer, what MeshSym emits, and when a
score must be `unknown`. Numerical implementations do **not** live here.

Nothing here is implemented in MeshSym yet.

## Principles

- **Composition + gating.** Pick a measure, force `unknown` on bad quality
  or clock error, fold edges into a regime. Do not grow a second SampEn.
- **Features up, raw on pull.** Default upward path is snapshots + quality.
  Higher layers may request longer windows or rawer series. Policy for
  *when* is out of scope; the capability is required.
- **Live path is causal.** No future window in a score that can drive a
  brief or cue. Post-hoc SpatialSym classifiers stay lab-only.
- **No single “coherence” score.** Edge types stay distinct.

## Layer → crate

| Layer | MeshSym emits | Call (do not reimplement) |
|:------|:--------------|:--------------------------|
| **L0** | Quality bits; optional tiny complexity proxy | `symworx-math::series` (successive differences). Keep MCU compute tiny. |
| **L1** | Person-now / sensor-now snapshot + quality | BioSym: short HRV, cadence, still/move, gait/PPG/resp quality presets. `symworx-dynamics::entropy` for short-window entropy. `symworx-stats` variability / basic descriptors. |
| **L2** | Person-pattern features | `symworx-dynamics` RQA + entropy on the longer series. |
| **L3** | Pairwise matrix + quality | Physio CRQA / movement CRQA: `symworx-dynamics::rqa`. Relative phase / cluster-phase inputs: `symworx-dynamics::phase`. Circular stats: `symworx-math::circular`. Causal IMU effort-phase: MeshSym **calls** a 1-D helper (do not fork SpatialSym’s XY API). Proximity class is MeshSym (ordinal RSSI). RSA / cardio-locomotor: BioSym when it exists. |
| **L4** | Regime label + cited features + brief | Cluster-phase (`ρ_group`, `ρ_k`): `symworx-dynamics::phase` (already shipped). MdRQA / CRQA **network** summaries: still dynamics notes. MeshSym maps components + coupling → together / intended split / unintended fragment / unknown. |

Cross-layer CRQA (person vs unit, movement vs proximity) is L3/L4, not a
sixth box.

Immediate cues sit on **L0/L1**. Formation L3/L4 windows are too slow to
drive haptic/visual feedback. A later closed loop feeds
`symworx-dynamics::control` / `symworx-signal` Kalman only with
quality-gated features.

## Quality

Dropout, clip, IMU energy, clock error, or RSSI flicker → the affected
score is `unknown`, not a number. Strap slip vs true decoupling must not
share a score. Missing heading → no directional phase (XY heading stays
SpatialSym).

## Request for more detailed data

Higher layers may ask a personal node or L0 for longer feature windows,
higher-rate segments, or extra cross measures. Pulled data travels with
quality flags and a schema version. Default remains the narrow feature bus.

## Where the methods are documented

| Topic | Home |
|:------|:-----|
| Cluster-phase, relative phase, MdRQA / CRQA networks | [`symworx-dynamics` group-phase notes](../../symworx-dynamics/notes/group-phase-coherence.md) |
| XY effort / directional phase (post-hoc, meters) | [`symworx-spatialsym` phase notes](../../symworx-spatialsym/notes/phase-coherence.md) |
| PPG, respiration, gait, HRV, RSA (gap), streaming analysis (gap) | [`symworx-biosym` README](../../symworx-biosym/README.md) |
| Series primitives | `symworx-math::series` |
| RMSSD / basic stats | `symworx-stats` |

## Non-goals

- Reimplementing RQA, CRQA, entropy, or SpatialSym XY kinematics here.
- Medical diagnosis, readiness-for-duty, or fitness claims.
- Putting Arduino / Pi HAL in this crate so a measure can “run on device.”

## See also

- [architecture.md](architecture.md)
- [design.md](design.md)
- [contracts.md](contracts.md)
- [glossary.md](glossary.md)
