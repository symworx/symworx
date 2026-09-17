# Glossary

Locked terms. Prefer these in the README, contracts, and (when it exists) code.

| Term | Meaning |
|:-----|:--------|
| **MeshSym** | This crate: live coupling analysis on a typed graph. Not SpatialSym, BioSym, or embed. |
| **body graph** | Many L0 channels on one `sid` (e.g. IMU, PPG, belt). |
| **formation graph** | Personal nodes as vertices (one person each). |
| **personal node** | Box or process paired to one person. Hub of that person’s body graph; member of a formation graph. |
| **unit node** | Box or process that holds the coupling matrix for one formation. |
| **primary pair** | Exclusive link person-device ↔ personal node. |
| **L0–L4** | Sample → person-now / sensor-now → person-pattern → pair → regime. See [architecture.md](architecture.md). |
| **brief** | Compact downward message: cited features + relative status, not raw series. |
| **relative status** | Local picture of self + neighbors (or sibling sensors) + last unit state, with ages. |
| **degraded picture** | Relative status when the unit node (or a sibling sensor) is unreachable. |
| **physio coupling** | Pairwise ANS/HRV relation (e.g. CRQA). |
| **movement coupling** | Pairwise IMU/envelope or kinematic relation. |
| **proximity edge** | Reciprocal radio-nearness class (not XY). Body-graph placement is anatomy, not RSSI. |
| **effort phase** | Non-directional accel/decel timing (both go / both brake / opposed). Causal on the live path. |
| **directional phase** | Effort plus heading or along-track sign. Needs a heading source; XY heading stays SpatialSym. |
| **regime** | State of the graph as one trajectory (order, components, mean coupling). |
| **together** | One proximity (or placement) component, high movement coupling. |
| **intended split** | Multiple components *and* anti-phase movement (designed). |
| **unintended fragment** | Components appear and coupling collapses (or a channel drops on quality). |
| **quality** | Honesty flags (dropout, clip, IMU energy, RSSI flicker, clock error). |
| **feature bus** | Quality-gated numbers the unit node and later agents may see. Raw is not on this bus. |
| **`sid`** | Subject id (embed rule). Sensors get their own ids; do not conflate with `sid`. |

## Words we do not use as claims

| Avoid | Why |
|:------|:----|
| team sync | Vendors mean XY/tactics. |
| readiness | Whoop / OHWS individual scores. |
| group coherence | HeartMath’s product language. |
| map / position | We do not have trustworthy XY from RSSI. |
| efficiency (as a single index) | Use the three cartoons instead. |
| player / pitch / ball | SpatialSym public types stay sport-agnostic; this crate is not XY. |
