# Lineage

Working history. Durable claims live in [design.md](design.md) and
[architecture.md](architecture.md).

Earlier private field-kit notes used the name **SymMesh** for a cSYMd
personal-node / unit-node kit. That material is rewritten here as
**MeshSym** (`symworx-meshsym`): an Apache-2.0 analysis crate in the
SymWorx workspace.

What carried over:

- Typed edges (physiology, movement, proximity), not a roster of scores.
- Quality gates pairing math (bad RSSI is not a split).
- Features up; raw on pull; briefs down.
- Personal node still useful if the hub is gone (degraded picture).
- Math in existing crates; wrap, do not fork.
- First analysis proof is synthetic L3, not a cluster of watches.

What changed in the rewrite:

- Product and crate name: MeshSym / `symworx-meshsym`.
- License and home: public workspace, not a private kit repo.
- Body graph (many L0 per `sid`) is a first-class instantiation, not an
  afterthought on “one sensor per user.”
- SpatialSym is the XY / GPS post-hoc cousin, not the parent crate.
- Firmware, radios, and apps stay out of tree. Empty reserved node
  directories were not copied.

Deferred until a feature bus exists: Flutter, Tailscale, dual-persona BLE,
honey sessions, card generators.

Code order when implementation starts: contracts → synthetic L3 → causal
effort-phase → L4 label. Hardware after that.
