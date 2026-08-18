# Projects

Living list of work **slated** for development and items only **under
consideration**. Not a schedule or commitment. Status here is informal;
crate READMEs and [CHANGELOG.md](../CHANGELOG.md) stay the source of
truth for what already shipped.

When an item grows design notes, keep those next to the crate (or under
`docs/`) and link them from here. Do not treat this file as an
implementation spec.

## Slated

Called out in existing docs as planned, partial, or paused with intent
to finish.

| Area | Item | Notes |
|:------|:------|:-------|
| Dynamics | Full CRQA API | README marks CRQA as **Planned**. Pairwise / cross-recurrence exists in pieces; a complete public API does not. |
| Stats | Welch PSD (`stats::spectral`) | Stub / placeholder. Do not rely on it for analysis until replaced. |
| Python | Broader `core.statistics` surface | Rust-first today. Still unbound: LMM, rule lists, k-NN, LDA, polyreg, full OLS/ridge objects. |
| Release | Re-enable PyPI publish | Trusted publishing + `publish-python` in [release.yml](../.github/workflows/release.yml). Validation already builds a wheel smoke artifact. |
| SpatialSym | Sport-specific reporting wrappers | Core stays sport-agnostic. Named wrappers for analysis/presentation are planned on top ([spatialsym README](../crates/symworx-spatialsym/README.md)). |

## Considered

Candidates and research notes. No implementation plan unless a linked
note says otherwise.

| Area | Item | Notes |
|:------|:------|:-------|
| Dynamics / SpatialSym | Group phase coherence | Cluster-phase, MdRQA / pairwise CRQA networks, relative phase. Literature: [group-phase-coherence.md](../crates/symworx-dynamics/notes/group-phase-coherence.md). SpatialSym application: [phase-coherence.md](../crates/symworx-spatialsym/notes/phase-coherence.md). |
| SpatialSym | Accel / decel as pairwise (and group) phase | Today's APIs **count** speed-ups and slow-downs. A later pass could score in-phase vs out-of-phase effort and heading. Design notes: [phase-coherence.md](../crates/symworx-spatialsym/notes/phase-coherence.md). |
| BioSym | Advanced physiology metrics | Waveform morphology, extended HRV, cardiorespiratory coupling / RSA, sleep sim, real-sensor quality presets, streaming analysis ([biosym README](../crates/symworx-biosym/README.md)). |
| TUI | Explore / Dynamics depth | Filtering, processing, embedding / FNN in Explore; more nonlinear tools in Dynamics ([AGENTS.md](../AGENTS.md)). TUI smoke build in CI is still off. |
| Backend | Real process supervision | `symworx-backend` is experimental; `ProcessManager` is a placeholder API. |
| Bindings | R package | Stub only. |
| Stats / signal | Spectral / coupling follow-ons | Transfer entropy and related directional measures are secondary to first-line coherence ([group-phase notes](../crates/symworx-dynamics/notes/group-phase-coherence.md)). |

## How to update

- **Slated** — something we have already named as planned or paused-to-finish in a README, changelog, or workflow.
- **Considered** — useful, undecided, or research-only. Move to slated when it has an intended crate home and a near-term slice.
- Drop a row when the work ships; do not keep a done list here (use the changelog).
