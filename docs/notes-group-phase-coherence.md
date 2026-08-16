# Notes — Group phase coherence, in-phase / out-of-phase

Reference notes for measures relevant to multi-person / formation coupling (SymMesh unit regime and related work). Not an implementation plan. Math stays in the dynamics / RQA / SpatialSym crates; these are candidate summaries and literature anchors.

## Why this note exists

SymMesh and similar field kits need coherence and in-/out-of-phase measures that:

- scale from dyads to small groups (~2–12),
- support both oscillatory and non-oscillatory series,
- distinguish **together** / **intended split** / **unintended fragment**,
- keep univariate regularity and crossed/multi regularity explicit at every layer.

## Primary recommendations

### 1. Cluster-phase method (Frank & Richardson)

Best first-line group phase-coherence measure for multivariate movement (and other phase-extractable) series.

- **ρ_group** ∈ [0, 1]: overall group synchrony (Kuramoto-based order parameter).
- **ρ_k** per individual: strength of locking to the group cluster phase.
- Mean and SD of relative phase to the cluster phase → distinguishes **in-phase** (~0°), **anti-phase** (~180°), or other stable relations.
- Naturally surfaces multiple clusters → supports intended split vs unintended fragment.

**Key references**

- Frank, T. D., & Richardson, M. J. (2010). On a test statistic for the Kuramoto order parameter of synchronization. *Physica D*.
- Richardson, M. J., Garcia, R. L., Frank, T. D., Gergor, M., & Marsh, K. L. (2012). Measuring group synchrony: a cluster-phase method for analyzing multivariate movement time-series. *Frontiers in Physiology*, 3, 405. https://doi.org/10.3389/fphys.2012.00405

### 2. Multivariate / multidimensional RQA (MdRQA) and joint recurrence

Strong when series are not cleanly oscillatory (HRV/ANS, IMU envelopes, event-like series).

- Group-level recurrence measures, or
- Pairwise CRQA matrix → network summaries (mean/median DET, diagonal-line entropy, fraction of strong edges, component count / modularity).

These network / MdRQA summaries become the “group descriptors” that can themselves receive entropy or RQA at the unit-regime layer.

**Key references**

- Wallot, S., Roepstorff, A., & Mønster, D. (2016). Multidimensional Recurrence Quantification Analysis (MdRQA) for the analysis of multidimensional time-series. *Frontiers in Physiology*.
- Shockley, K., et al., and the broader interpersonal CRQA literature (Richardson, Dale, Riley, Paxton, sports CRQA papers).

### 3. Supporting measures

| Goal | Measure | Notes |
|------|---------|-------|
| Continuous global coherence | Kuramoto order parameter *R* | Simple baseline when phases are reliable |
| Fine-grained in-/out-of-phase | Continuous relative phase (Hilbert / wavelet) + circular statistics (mean resultant length) | Pairwise or vs cluster phase |
| Effort vs direction | Effort-phase (non-directional) vs directional-phase | Already in SpatialSym design notes; keep the distinction |
| Directional influence (later) | Transfer entropy / cross-entropy style | Secondary; not first-line “coherence” |

## Layer mapping (SymMesh-oriented)

- **L1 / L2 (individual):** univariate entropy / RQA of the person’s series; short-window complexity for possible immediate feedback.
- **L3 (pairwise):** physio CRQA, movement CRQA, relative phase / phase locking, effort vs directional phase, pairwise cross measures.
- **L4 (unit regime):** cluster-phase (ρ_group, ρ_k, φ to cluster), network summaries of the CRQA matrix, MdRQA / joint RQA on group descriptors, entropy of those descriptors → regime label.

Univariate and crossed/multi regularity are both assessed at every layer; higher layers may request more detailed series under conditions defined elsewhere.

## Practical rules

- Extract phases only when the signals support it (clear oscillatory structure or after appropriate filtering / Hilbert). Otherwise lean on MdRQA / pairwise-CRQA + network path.
- Quality gates can force any phase or coupling score to `unknown`.
- Do not collapse all edge types into a single “coherence” or “sync” product score.

## See also

- SymMesh `docs/analytics.md` and `docs/diagrams.md` (csymd/symmesh) for the architectural contract that consumes these measures.
- SpatialSym kinematics notes for effort-phase vs directional-phase.
