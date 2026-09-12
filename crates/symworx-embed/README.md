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

## Citation

If you use this software, please cite **SymWorx**, not this crate alone.
Name the crate in the paper if it helps; the bibliography title is still *SymWorx*.

Berry, N. (2026). *SymWorx* [Computer software]. https://github.com/symworx/symworx

```bibtex
@software{Berry_SymWorx_2026,
  author  = {Berry, Nate},
  license = {Apache-2.0},
  title   = {{SymWorx}},
  url     = {https://github.com/symworx/symworx},
  year    = {2026}
}
```

Add the version you used. [`CITATION.cff`](https://github.com/symworx/symworx/blob/main/CITATION.cff) and GitHub **Cite this repository** carry the current release.
