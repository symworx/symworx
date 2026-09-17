# Related work

What already exists, and the slice this crate is aimed at. Not a literature
review.

## Live many-person physiology

**Zephyr, Equivital, Polar Team, Firstbeat, Whoop Teams.**
Roster of individual vitals or load. “Team” means aggregates and comparisons.
Closest operational cousin: Zephyr’s squad status grid.

**Difference:** typed *edges* and a regime trajectory, not a grid of hearts.

## Team geometry

**Catapult, STATSports, Kinexon LPS, video tracking.**
Real XY: stretch, compactness, formation. SpatialSym sits in this family
when you have positions.

**Difference:** no stadium install in v0. RSSI is topology, not a pitch
diagram. GPS / LPS analysis stays in SpatialSym.

## Interpersonal coupling (methods)

**Shockley, Richardson, Dale, Riley, Paxton** and sports CRQA papers.
Gold-standard pairwise CRQA, mostly dyads, lab, offline.

**Difference:** fielded or live, quality-gated, more than two nodes, three
edge types, briefs. The *math* is not new; it lives in `symworx-dynamics`.

## Intrapersonal / cross-modal coupling

Cardiorespiratory coupling, RSA, cardio-locomotor coupling. BioSym’s CPG
already *models* heart–legs–respiration; dedicated live cross-analysis is
still a BioSym gap. MeshSym is the graph those measures can hang on (body
instantiation).

## Proximity without sport geometry

**BLE contact tracing, SocioPatterns, badge studies.**
Proximity or interaction networks.

**Difference:** proximity is one edge next to physio and movement; the
graph trajectory is the object.

## Group HRV products

**HeartMath** “group coherence.” Training/biofeedback; not IMU, not RSSI
graphs, not RQA on a unit descriptor.

Do not reuse “coherence” as a product word.

## Defense readiness wearables

**OHWS, H2F, POTFF, Whoop/Oura on soldiers.**
Individual sleep/strain/recovery. TAK-class systems mesh location and C2,
not coupling.

**Difference:** “is this fire team one system or five,” in training — not a
sleep score. Same non-claims as the rest of SymWorx.

## Host streaming in this workspace

`symworx-embed` is a single PPG JSON-line pipe (`sid`, serial/simulator).
It established a live host path. MeshSym is the coupling graph, not a
second PPG protocol.

## SpatialSym kinematics

`symworx-spatialsym` derives XY velocity, path linearity, and
effort / directional phase **post-hoc** (bidirectional windows). That work
stays there. MeshSym uses **causal** IMU effort-phase on the movement edge
and does not fork the XY crate.
