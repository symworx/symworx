# Notes — Group phase coherence, in-phase / out-of-phase

Shipped in `symworx-dynamics::phase`: relative phase, Kuramoto *R*, cluster-phase
(`ρ_group`, `ρ_k`, mean relative phase). MdRQA / CRQA networks are still notes only.

Reference notes for multi-person / multi-series phase coherence and related
measures. Not an implementation plan. Literature anchors for cluster-phase,
MdRQA, and CRQA networks — methods that would live in this crate if implemented.
SpatialSym is a consumer, not the home of these measures.

SpatialSym kinematics application (effort-phase vs directional-phase, accel/decel
pairs, where a first cut would land):

[`crates/symworx-spatialsym/notes/phase-coherence.md`](../../symworx-spatialsym/notes/phase-coherence.md)

## Purpose

Useful when analyzing groups of time series (movement, physiology, or mixed)
where the questions include:

- overall group synchrony,
- individual locking strength to a group rhythm,
- in-phase vs anti-phase (or other stable relative-phase) relations,
- detection of multiple clusters / subgroups.

Both oscillatory and non-oscillatory series matter; the choice of method should
follow the signal structure.

## Primary recommendations

### 1. Cluster-phase method (Frank & Richardson)

Strong first-line measure for multivariate movement (and other phase-extractable)
series.

- **ρ_group** ∈ [0, 1]: overall group synchrony (Kuramoto-based order parameter).
- **ρ_k** per individual: strength of locking to the group cluster phase.
- Mean and SD of relative phase to the cluster phase → distinguishes **in-phase**
  (~0°), **anti-phase** (~180°), or other stable relations.
- Naturally surfaces multiple clusters.

**Key references**

- Frank, T. D., & Richardson, M. J. (2010). On a test statistic for the Kuramoto
  order parameter of synchronization. *Physica D*.
- Richardson, M. J., Garcia, R. L., Frank, T. D., Gergor, M., & Marsh, K. L.
  (2012). Measuring group synchrony: a cluster-phase method for analyzing
  multivariate movement time-series. *Frontiers in Physiology*, 3, 405.
  https://doi.org/10.3389/fphys.2012.00405

### 2. Multivariate / multidimensional RQA (MdRQA) and joint recurrence

Prefer when series are not cleanly oscillatory (e.g. HRV/ANS, IMU envelopes,
event-like series).

- Group-level recurrence measures, or
- Pairwise CRQA matrix → network summaries (mean/median DET, diagonal-line
  entropy, fraction of strong edges, component count / modularity).

These summaries can themselves be treated as higher-order descriptors (and later
receive entropy or RQA if desired).

**Key references**

- Wallot, S., Roepstorff, A., & Mønster, D. (2016). Multidimensional Recurrence
  Quantification Analysis (MdRQA) for the analysis of multidimensional
  time-series. *Frontiers in Physiology*.
- Shockley, K., and the broader interpersonal CRQA literature (Richardson, Dale,
  Riley, Paxton, and sports CRQA papers).

### 3. Supporting measures

| Goal | Measure | Notes |
|------|---------|-------|
| Continuous global coherence | Kuramoto order parameter *R* | Simple baseline when phases are reliable |
| Fine-grained in-/out-of-phase | Continuous relative phase (Hilbert / wavelet) + circular statistics (mean resultant length) | Pairwise or vs a cluster phase |
| Effort vs direction | Effort-phase (non-directional) vs directional-phase | SpatialSym kinematics; see crate note |
| Directional influence (later) | Transfer entropy / cross-entropy style | Secondary; not first-line “coherence” |

## Practical rules

- Extract phases only when the signals support it (clear oscillatory structure or
  after appropriate filtering / Hilbert). Otherwise lean on MdRQA / pairwise-CRQA
  + network summaries.
- Quality / validity checks can force any phase or coupling score to an unknown /
  invalid state rather than a misleading number.
- Prefer keeping edge types (or modality types) distinct; avoid collapsing
  everything into a single undifferentiated “coherence” score unless that is the
  explicit research question.

## See also

- SpatialSym: [`crates/symworx-spatialsym/notes/phase-coherence.md`](../../symworx-spatialsym/notes/phase-coherence.md)
- Existing RQA / CRQA / entropy implementations in this crate.
