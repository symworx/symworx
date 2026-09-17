# Contracts

Language-agnostic object shapes. Specify here **before** ingest or firmware.

No JSON Schema or code in this pass. When fields stabilize, add schema
files next to this note and tests against synthetic fixtures.

## Domain (this crate)

Semantic objects MeshSym owns. Other crates serialize or transport them;
they do not redefine regime or RSSI class.

### Snapshot (up)

Emitted by a personal node on a short interval (about 1–2 s) or on session
close. Many L0 channels per `sid` are allowed.

| Field | Intent |
|:------|:-------|
| `node_id`, `sid` (opaque) | identity; `sid` is a subject id, not a name |
| `channel_id` (optional) | L0 sensor / modality on that `sid` |
| `session_id`, `t` | session + timestamp (time-lord clock once synced) |
| `quality` | dropout, clip, IMU energy, clock error, RSSI flicker |
| `physio` | short HR / rMSSD / flags — not the RR list by default |
| `movement` | cadence, intensity, still/move — not raw IMU |
| `rssi` | list of `{peer_id, rssi_class, last_heard}` reciprocal when possible |
| `schema` | version string |

### Edge (internal)

Per window, per pair, per type: physio coupling, movement coupling,
proximity class (formation) or placement class (body). Score is a number
**or** `unknown`.

### Graph matrix (internal)

Per window: the edge set plus L4 descriptors (components, order, mean DET /
in-phase fraction).

### Brief (down)

| Field | Intent |
|:------|:-------|
| `t`, `age` | when computed, how stale |
| `regime` | together / intended split / unintended fragment / unknown |
| `self` | this node’s quality + degree + cited metrics |
| `neighbors` | compact relative status (not waveforms); sibling sensors on a body graph |
| `citations` | feature ids a later card is allowed to mention |

A self-only brief (body graph, feedback) is valid: cue + which edges are
stale. Formation fields (RSSI class, neighbor list) are optional, not core.

### Quality

A missing heading or a flickering RSSI edge must make **directional phase**
and **proximity-as-split** `unknown`, not a number. Strap slip vs true
decoupling must not share a score.

## On disk (`symworx-io`, later)

When snapshot / brief / synthetic-fixture **files** exist, read and write
them through `symworx-io` (`SymReader` / `SymWriter` or a mesh-specific
module there). Schema version lives in the file. Waveforms still only via
io. MeshSym does not take a parquet/csv dependency.

This crate does not own L0–L4 or the meaning of a regime just because a
file is a JSON snapshot.

## On the wire (Arduino ↔ node, later)

The same domain objects. Arduino-class L0 talks **bytes + schema version +
quality bits**, not a Cargo dependency on MeshSym. Live PPG JSON-line stays
`symworx-embed` until there is a distinct mesh line; do not overload that
protocol. A Pi node daemon may use MeshSym as a library; the MCU must not.

## Non-goals for v0

- Raw IMU/ECG in the default up-path
- Names, emails, or jersey numbers in the snapshot
- A single “sync score”
- RSSI converted to meters
