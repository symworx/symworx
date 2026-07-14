# symworx-spatialsym

Spatial trajectory analysis and post-hoc movement decision modeling.

**Sport-agnostic by design.** No soccer (or other sport) terminology in public APIs.

## Current status

- `Point2` / `Vec2` with arithmetic, `bearing`, distances
- `Trajectory` + validation
- Velocity derivation (`derive_velocities` / `derive_velocities_from_times`)
- Pairwise and focal distances
- `PlayingDimensions` (rectangular field, meters)
- `SpaceAction`: Expansion, Penetration, Denial, Pressure, Neutral
- Errors via `SpatialError` + `Result`

Analyses are **post-hoc** labels of agent decisions around space (see `decision` module).
All lengths are **meters**.

## Usage

```rust
use symworx_spatialsym::{
    Point2, SpaceAction, PlayingDimensions,
    pairwise_distances, derive_velocities,
};

let pts = vec![Point2::new(0., 0.), Point2::new(1., 0.)];
let dists = pairwise_distances(&pts);
let vels = derive_velocities(&pts, 0.1); // fixed dt (s)

let _dims = PlayingDimensions::new(105.0, 68.0);
let action = SpaceAction::Penetration;
println!("{}", action.description());
```
