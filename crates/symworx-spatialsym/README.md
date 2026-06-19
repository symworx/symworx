# symworx-spatialsym

Spatial trajectory analysis and post-hoc movement decision modeling.

**Sport-agnostic by design.** No soccer (or other sport) terminology in public APIs.

## Current Status (Phase 1 complete)

- `Point2` / `Vec2` with full arithmetic, `bearing` (atan2), normalization, distances.
- `Trajectory` container + validation.
- Velocity derivation (forward differences / dt) — uses time explicitly (no 30 fps hardcode).
- Pairwise distances and focal distances.
- `ArenaSpec` (rectangular, meters).
- `SpaceAction` enum: `Expansion`, `Penetration`, `Denial`, `Pressure`, `Neutral` (locked per review).
- Errors via `SpatialError` + `Result`.

## Philosophy

Analyses are performed **post-hoc** using both historical and future windows on trajectories to label agent decisions around space:
- Expansion (create)
- Penetration (exploit)
- Denial (deny)
- Pressure (close)

See `decision` module and future `classify_*` functions.

## Usage

```rust
use symworx_spatialsym::{Point2, Vec2, pairwise_distances, derive_velocities_from_times, SpaceAction, ArenaSpec};

let pts = vec![Point2::new(0.,0.), Point2::new(1.,0.)];
let dists = pairwise_distances(&pts);
let vels = derive_velocities(&pts, 0.1); // dt = 0.1s

let arena = ArenaSpec::new(105.0, 68.0); // meters, generic
let action = SpaceAction::Penetration;
println!("{}", action.description());
```

## Next

See the project plan (plan.md in session) for Phase 2+ (bidirectional decision classifiers, better time-aware kinematics, CSV loading, etc.).

All lengths are **meters**.
