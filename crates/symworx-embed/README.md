# symworx-embed

Host-side streaming for embedded biosignal devices.

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
|:--------|:--------|:--------|
| `simulate` | yes | Synthetic vitals source |
| `serial` | no | Serial port reader (`serialport`, no libudev — open by path) |

## Examples

```bash
# Synthetic stream
cargo run -p symworx-embed --example simulate_print
cargo run -p symworx-embed --example simulate_print -- --sid S002 --n 20

# Arduino
cargo run -p symworx-embed --features serial --example serial_dump -- \
  --port /dev/ttyACM0 --sid S001
```

## Library surface

- `StreamSample`, `SourceKind`, `sid`
- `parse_json_line` / `sample_to_json_line` / `enrich`
- `SampleRing` + `Channel`
- `StreamSource` trait
- `SimulatorSource` / `SerialSource`
- `analyze_vitals` (threshold status)
