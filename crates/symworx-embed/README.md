# symworx-embed

Host-side streaming for embedded biosignal devices (SentryWard-style PPG).

Phase 1 is **host-first**: parse the Arduino JSON-line protocol, simulate or
read serial streams, and feed ring buffers for analysis / a future TUI live
mode. Device firmware stays in SentryWard’s Arduino sketch for now; Embassy
firmware is a later phase.

## Terminology

SymWorx uses **subject** naming, not patient:

| Concept | Wire / API field |
|---------|------------------|
| Subject id | **`sid`** |
| Legacy SentryWard ingress only | `patient_id` (accepted, never emitted) |

## Wire protocol

Device line (~50 Hz, 115200 baud):

```json
{"red":12345,"ir":23456,"bpm":72.3,"bpm_avg":71,"ts":123456}
```

Host-enriched line:

```json
{"sid":"S001","source":"arduino","red":12345,"ir":23456,"bpm":72.3,"bpm_avg":71,"ts":123456}
```

## Features

| Feature | Default | Purpose |
|---------|---------|---------|
| `simulate` | yes | Synthetic vitals source |
| `serial` | no | Serial port reader (`serialport`, no libudev — open by path) |

## Examples

```bash
# Synthetic stream
cargo run -p symworx-embed --example simulate_print
cargo run -p symworx-embed --example simulate_print -- --sid S002 --n 20

# Real Arduino (SentryWard medsym-sensor sketch)
cargo run -p symworx-embed --features serial --example serial_dump -- \
  --port /dev/ttyACM0 --sid S001
```

### Serial permissions (Silverblue / toolbox)

1. Host: `sudo usermod -a -G dialout $USER` (log out/in once).
2. Enter toolbox with the device: `toolbox enter dev-web -- --device /dev/ttyACM0`
3. Run `serial_dump` as above.

## Library surface

- `StreamSample`, `SourceKind`, `sid`
- `parse_json_line` / `sample_to_json_line` / `enrich`
- `SampleRing` + `Channel`
- `StreamSource` trait
- `SimulatorSource` / `SerialSource`
- `analyze_vitals` (threshold status)

## Non-goals (this crate, for now)

- Embassy / `no_std` firmware
- Django / Redis / multi-bed clinical DB
- Heavy linear algebra or Polars
