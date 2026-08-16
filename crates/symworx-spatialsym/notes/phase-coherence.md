# SpatialSym — phase coherence (effort, heading, group)

Status: design / research notes. Not an implementation plan and **not** a shipped API.
Date: 2026-08-16

Consolidates the former crate note on accel/decel phase with the SpatialSym-facing
parts of the group-phase notes. Cluster-phase / MdRQA / CRQA literature lives next
to the dynamics crate:
[`group-phase-coherence.md`](../../symworx-dynamics/notes/group-phase-coherence.md).

Analyses stay **post-hoc** (past + future windows), sport-agnostic in public names,
and in **meters / m/s / m/s²**.

## Why

`count_accelerations_decelerations` (and `AgentTrajectories::accel_decel_counts`)
only **count** scalar speed-ups and slow-downs per agent. That is useful load, but
it cannot say whether two agents are changing speed **together** or **against**
each other, or whether those changes share a heading.

Treat accel/decel as a **pairwise (and later group) phase** problem, in two
complementary senses:

1. **Non-directional (effort-phase)** — timing of effort only (speed change),
   ignoring heading.
2. **Directional (directional-phase)** — timing of effort **and** heading /
   along-track sign.

Both should report **in-phase** vs **out-of-phase**. Do not collapse them into a
single undifferentiated “coherence” score unless that is the explicit question.

## What exists today

| Piece | What it does | Gap |
|---|---|---|
| `kinematics::derive_speeds` | Scalar speed per interval | No signed along-track accel series |
| `kinematics::count_accelerations_decelerations` | Thresholded `Δspeed/Δt` event counts | No time series of events; no pairs |
| `kinematics::derive_velocities` / bearings | 2-D velocity and heading | Not combined with accel events |
| `PlayerSummary` / `GroupSummary` | Per-agent and summed accel/decel counts | Group = sum of individuals, not coupling |
| `metrics::pairwise_distances` | Positions only | No pairwise kinematic coupling |

Scalar `a = Δspeed / Δt` is already **non-directional in space** (speed has no
heading). It is still **signed in effort** (accel vs decel). Phase work should
keep that distinction.

## Layers

| Layer | Question | First-line measure |
|---|---|---|
| Pairwise effort | Do *i* and *j* change speed together? | Sign / event agreement of `a_scalar` |
| Pairwise directional | Same effort **and** similar heading / axis? | `a_along` (or other projection) + heading gate |
| Group (later) | Is there a shared rhythm? Who locks to it? | Cluster-phase (`ρ_group`, `ρ_k`) on extracted phases |
| Non-oscillatory fallback | Coupling when phases are not trustworthy | MdRQA, or pairwise CRQA → network summaries |

Extract phases only when the series support it (clear oscillatory structure, or
after appropriate filtering / Hilbert). Otherwise lean on MdRQA / pairwise CRQA.
Quality / validity checks can force any phase or coupling score to unknown rather
than a misleading number.

## Pairwise: in-phase vs out-of-phase

For agents `i` and `j` in a shared time window:

**Non-directional (effort timing)**

- **In-phase:** both accelerate, or both decelerate, in the same window (or within
  a small lag).
- **Out-of-phase:** one accelerates while the other decelerates.

This is the “do they change effort together?” layer. It does not care if they face
opposite ways. A line stepping together, or two people hitting the brakes
together, should score in-phase even if headings differ.

**Directional (effort + heading)**

Project each agent’s acceleration onto a reference direction, then compare signs
/ phase. Useful references (pick per call; do not bake in sport language):

- each agent’s own heading (along-track `a`)
- bearing of `j` relative to `i` (closing vs opening)
- a shared axis (sideline, attack direction, `PlayingDimensions` long axis)

Name the two directional failure modes separately so they are not collapsed:

| | Same heading | Opposite heading |
|---|---|---|
| Same effort sign (both accel or both decel) | directional in-phase | effort in-phase, **spatially opposed** |
| Opposite effort sign | directional out-of-phase (one go, one brake) | mixed; treat as out-of-phase unless a later rule says otherwise |

Non-directional analysis is the first column-agnostic collapse (ignore heading).
Directional analysis keeps the table.

## Suggested series (do not implement in this note)

Per agent, aligned to the same time base as speeds (length `n-1` or a documented
resample):

- `speed[t]`, `a_scalar[t] = Δspeed/Δt` (already implied by the counter)
- event flags: `accel_event`, `decel_event` at `accel_threshold` (reuse the
  existing threshold)
- `heading[t]`, `a_vec[t]` from successive velocities
- `a_along[t] = a_vec · unit(heading)` (signed along-track)
- optional `a_close[t] = a_vec · unit(other - me)` (signed closing accel)

Pairwise, in a window of `W` seconds (same bidirectional style as `past_bearings`
/ `future_bearings`):

- coincidence of event flags (non-directional in-phase / out-of-phase counts or
  time fraction)
- sign agreement of `a_scalar` and of `a_along` (continuous, not only events)
- circular heading difference; gate directional scores when `|Δheading|` is
  undefined (near-zero speed)
- optional lag: allow ±`τ` so a slightly late partner still counts in-phase

## Group roll-up (later)

Do **not** replace `GroupSummary` totals; add coupling fields beside them.

When phases are reliable:

- **ρ_group** ∈ [0, 1]: overall group synchrony (Kuramoto-based order parameter).
- **ρ_k** per individual: strength of locking to the group cluster phase.
- Mean and SD of relative phase to the cluster phase → in-phase (~0°), anti-phase
  (~180°), or other stable relations.
- Naturally surfaces multiple clusters / subgroups.

Simpler first cut: mean pairwise in-phase fraction, or an order parameter on
signed `a_scalar` / `a_along`. Cluster-phase is the stronger first-line measure
once phases exist.

When series are not cleanly oscillatory (IMU envelopes, event-like flags, mixed
physiology + movement):

- group-level MdRQA, or
- pairwise CRQA matrix → network summaries (mean/median DET, diagonal-line
  entropy, fraction of strong edges, component count / modularity).

Prefer keeping edge types (or modality types) distinct.

Transfer entropy / cross-entropy style directional influence is secondary — not
first-line “coherence”.

## Quality / clocks

- Same rule as other pairwise kinematics: if clock error or missing samples
  exceed the event width, do not emit a phase score.
- Near-zero speed: no heading → directional metrics are `None`; non-directional
  may still be valid.
- RSSI / IMU-only kits (no XY) can use **non-directional** phase from a strap IMU
  magnitude; directional needs velocity or a heading source. Keep that split in
  the API so a later field path can call the first without faking the second.

## Where to extend (when this is implemented)

1. `src/kinematics.rs` — series + pairwise phase helpers (keep counts as-is).
2. `src/metrics.rs` — frame- or window-wise pair summaries next to
   `pairwise_distances`.
3. `src/trajectory.rs` — optional fields on summaries; do not break `accel_count`
   / `decel_count`.
4. Tests: two agents, known together / opposed / one-go-one-brake drills
   (synthetic trajectories already exist).

Group-level cluster-phase / MdRQA would likely call into `symworx-dynamics`
rather than reimplement RQA here.

## Out of scope for a first cut

- Real-time classification
- Replacing `SpaceAction` labels (phase can later inform Penetration vs
  Expansion, but that is a decision-layer follow-on)
- Treating RSSI as heading
- A single collapsed “group coherence” number that mixes effort and heading

## Literature (anchors only)

- Frank, T. D., & Richardson, M. J. (2010). On a test statistic for the Kuramoto
  order parameter of synchronization. *Physica D*.
- Richardson, M. J., Garcia, R. L., Frank, T. D., Gergor, M., & Marsh, K. L.
  (2012). Measuring group synchrony: a cluster-phase method for analyzing
  multivariate movement time-series. *Frontiers in Physiology*, 3, 405.
  https://doi.org/10.3389/fphys.2012.00405
- Wallot, S., Roepstorff, A., & Mønster, D. (2016). Multidimensional Recurrence
  Quantification Analysis (MdRQA) for the analysis of multidimensional
  time-series. *Frontiers in Physiology*.
- Shockley, K., and the broader interpersonal CRQA literature (Richardson, Dale,
  Riley, Paxton, and sports CRQA papers).
