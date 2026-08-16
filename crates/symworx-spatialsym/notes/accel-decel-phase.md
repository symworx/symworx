# Accel / decel phase analysis (update note)

Status: planned analysis update. No API change in this note.
Date: 2026-08-16
Branch: `notes/spatialsym-accel-phase`

## Why

`count_accelerations_decelerations` (and `AgentTrajectories::accel_decel_counts`) only **count** scalar speed-ups and slow-downs per agent. That is useful load, but it cannot say whether two agents are changing speed **together** or **against** each other, or whether those changes share a heading.

Update the analysis to treat accel/decel as a **pairwise (and later group) phase** problem, in two complementary senses:

1. **Non-directional** — timing of effort only (speed change), ignoring heading.
2. **Directional** — timing of effort **and** heading / along-track sign.

Both should report **in-phase** vs **out-of-phase**. Analyses stay **post-hoc** (past + future windows), sport-agnostic in public names, meters / m/s / m/s².

## What exists today

| Piece | What it does | Gap |
|---|---|---|
| `kinematics::derive_speeds` | Scalar speed per interval | No signed along-track accel series |
| `kinematics::count_accelerations_decelerations` | Thresholded `|Δspeed/Δt|` event counts | No time series of events; no pairs |
| `kinematics::derive_velocities` / bearings | 2-D velocity and heading | Not combined with accel events |
| `PlayerSummary` / `GroupSummary` | Per-agent and summed accel/decel counts | Group = sum of individuals, not coupling |
| `metrics::pairwise_distances` | Positions only | No pairwise kinematic coupling |

Scalar `a = Δspeed / Δt` is already **non-directional in space** (speed has no heading). It is still **signed in effort** (accel vs decel). Phase work should keep that distinction.

## Target questions

For agents `i` and `j` in a shared time window:

**Non-directional (effort timing)**

- **In-phase:** both accelerate, or both decelerate, in the same window (or within a small lag).
- **Out-of-phase:** one accelerates while the other decelerates.

This is the “do they change effort together?” layer. It does not care if they face opposite ways. A line stepping together, or two people hitting the brakes together, should score in-phase even if headings differ.

**Directional (effort + heading)**

- Project each agent’s acceleration onto a reference direction, then compare signs / phase.
- Useful references (pick per call; do not bake in sport language):
  - each agent’s own heading (along-track `a`)
  - bearing of `j` relative to `i` (closing vs opening)
  - a shared axis (sideline, attack direction, `PlayingDimensions` long axis)
- **In-phase:** along-track (or projected) accels have the same sign, and headings are aligned within a tolerance (e.g. both speeding up along similar bearings).
- **Out-of-phase:** opposite along-track signs, **or** similar effort signs on **opposing** headings (both “accelerate” but away from each other).

Name the two directional failure modes separately so they are not collapsed:

| | Same heading | Opposite heading |
|---|---|---|
| Same effort sign (both accel or both decel) | directional in-phase | effort in-phase, **spatially opposed** |
| Opposite effort sign | directional out-of-phase (one go, one brake) | mixed; treat as out-of-phase unless a later rule says otherwise |

Non-directional analysis is the first column-agnostic collapse (ignore heading). Directional analysis keeps the table.

## Suggested series (do not implement in this note)

Per agent, aligned to the same time base as speeds (length `n-1` or a documented resample):

- `speed[t]`, `a_scalar[t] = Δspeed/Δt` (already implied by the counter)
- event flags: `accel_event`, `decel_event` at `accel_threshold` (reuse the existing threshold)
- `heading[t]`, `a_vec[t]` from successive velocities
- `a_along[t] = a_vec · unit(heading)` (signed along-track)
- optional `a_close[t] = a_vec · unit(other - me)` (signed closing accel)

Pairwise, in a window of `W` seconds (same bidirectional style as `past_bearings` / `future_bearings`):

- coincidence of event flags (non-directional in-phase / out-of-phase counts or time fraction)
- sign agreement of `a_scalar` and of `a_along` (continuous, not only events)
- circular heading difference; gate directional scores when `|Δheading|` is undefined (near-zero speed)
- optional lag: allow ±`τ` so a slightly late partner still counts in-phase

Group roll-up later: mean pairwise in-phase fraction, or an order parameter on signed `a_scalar` / `a_along`. Do not replace `GroupSummary` totals; add coupling fields beside them.

## Quality / clocks

- Same rule as other pairwise kinematics: if clock error or missing samples exceed the event width, do not emit a phase score.
- Near-zero speed: no heading → directional metrics are `None`; non-directional may still be valid.
- RSSI / IMU-only kits (no XY) can use **non-directional** phase from a strap IMU magnitude; directional needs velocity or a heading source. Keep that split in the API so the field mesh can call the first without faking the second.

## Where to extend

1. `src/kinematics.rs` — series + pairwise phase helpers (keep counts as-is).
2. `src/metrics.rs` — frame- or window-wise pair summaries next to `pairwise_distances`.
3. `src/trajectory.rs` — optional fields on summaries; do not break `accel_count` / `decel_count`.
4. Tests: two agents, known together / opposed / one-go-one-brake drills (synthetic trajectories already exist).

## Out of scope for the first cut

- Real-time classification
- Replacing `SpaceAction` labels (phase can later inform Penetration vs Expansion, but that is a decision-layer follow-on)
- Treating RSSI as heading
