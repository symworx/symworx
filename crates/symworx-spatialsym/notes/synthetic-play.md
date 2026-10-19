# SpatialSym — synthetic play and decision labels

The 3v3 drill (`generate_3v3_attack`) is a single ~3 s action at 10 Hz: six
agents walk toward frozen or tracked targets. It saturates the 12 m action
radius, so “nearby” ≈ everyone, and `generate_ground_truth` paints labels from
a named scenario string that is **not** derived from those trajectories.

The 3v3 drill stays as the small example (`path_and_phase`, Spatial `g`).
`generate_11v11_play` is the sequence used to stress team shape and the
classifier (Spatial `a`).

## What the 11v11 generator is

- 22 agents (G0 4-3-3 attacking +x, G1 4-4-2 attacking −x) on the FIFA 105×68 m
  pitch with IFAB markings.
- Default **180 s at 1 Hz** (tracking-grade GPS/LPS, not optical 25 Hz).
- Each frame, agents **seek formation slots** that rise/drop with a phase table
  (build-up → progress → box entry → turnover/counter → switch → reset).
- The ball follows a planted carrier; passes are 1 s linear flights (one
  in-flight sample at 1 Hz).
- Planted `SpaceAction` labels come from the **script** (role + phase +
  distance to ball/goal), not from the kinematics. That split is the evaluation.

It is not a physics engine, not stochastic, and not a claim about real
11v11 tactics. Speeds are capped (walk/jog/run/sprint); GK motion is slow;
jitter is a deterministic sin/cos wobble.

## Why 1 Hz / 3 minutes

- Long enough for several possessions and a visible hull that expands and
  compresses.
- Coarse enough that 0.5 s bearing windows collapse to one sample — which is
  an honest test of classifiers written against 10 Hz clips.
- Matches a common tracking export rate.

## Visualization contract

- **Defending team:** convex hull of all 11 (full outline, including GK).
- **Possessing team:** up to four compact triangles among the six nearest
  teammates to the ball (max edge 30 m).
- Possession flips the two encodings. All-pairs phase edges stay on small
  batches (≤8 agents) only.

## Classifier evaluation

`evaluate_space_actions` is cell-wise (agent × frame), no temporal tolerance.
The TUI seeds with window=2 s, proximity=15 m, look-ahead=2 s. The
`synthetic_decisions` example also reports the old 3v3 defaults (0.5 / 10 / 0.8).

Known structural limits live in `classifier_limitations()` — the important
ones for 11v11:

1. Anyone outside `proximity_radius` of the inferred carrier is Neutral, so
   planted far-winger Expansion / back-line Denial are mostly invisible.
2. Conversion is a near-goal speed/space heuristic, not a shot or goal.
3. `free_space_ahead` is mean distance to all agents, not a gap through the
   block.
4. A 2 m possession gate blanks the carrier during the in-flight pass sample.

On the default 180 s @ 1 Hz sequence (3982 agent×frame cells), cell-wise
accuracy is about **0.15** for both the TUI 1 Hz params (2 s / 15 m / 2 s)
and the old 3v3 defaults (0.5 s / 10 m / 0.8 s). Almost all predicted mass
is Neutral (~3576–3878 of 3982). Creation, Conversion, and Prevention are
never predicted. Expansion and Denial are planted on the far block / wide
players and almost never recovered. Penetration, when predicted, is fairly
precise (prec ≈ 0.8) but rare.

That is a diagnostic, not a model-quality target. Widening the radius helps
a little; it does not fix the far-off-ball / team-shape gap. The hull and
local-triangle view is there because the classifier currently cannot see
that shape.
