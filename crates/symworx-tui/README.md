# symworx-tui

**`symview`** — Terminal user interface for selecting, converting, and visualizing biosym / physiological signals in the Symworx ecosystem.

> **Status**: v0.1 (Early development)

## Features

- Interactive file browser for biosignal files
- Manual path input support
- File conversion: `.ibi` → CSV and `.parquet` → CSV (both via `symworx-io`). The TUI **must** use `symworx-io` for all signal file I/O. `polars/parquet` (or direct `parquet` deps) must never be used for loading — they create unresolvable version conflicts with the workspace's chosen Parquet stack. Optional Polars usage is strictly for in-memory DataFrame work after loading via `symworx-io`.
- Clean two-screen interface: **File Selection** ↔ **Visualization**
- Keyboard-driven (vim-friendly navigation)
- Designed for easy integration with the rest of the Symworx crates

## Quick Start

### Launch the TUI

```bash
cargo run -p symworx-tui --bin symview
```

The TUI will look for signal files (`.csv`, `.txt`, `.dat`, `.ibi`, `.parquet`, `.biosym`) in `./data` and the current directory.

### Generate Example Data for Testing

Here are easy ways to create test files you can immediately load in `symview`.

#### Option 1: Simple synthetic signal (recommended for quick testing)

Create a file called `generate_test_data.py`:

```python
import numpy as np

# Generate a simple but realistic-looking physiological time series
fs = 100.0                    # sampling rate (Hz)
duration = 60.0               # seconds
t = np.arange(0, duration, 1/fs)

# Create a signal with a few frequency components + noise (mimics HRV or PPG-like data)
signal = (
    1.0 * np.sin(2 * np.pi * 0.1 * t) +           # slow baseline wander
    0.6 * np.sin(2 * np.pi * 1.2 * t) +           # main oscillation
    0.3 * np.sin(2 * np.pi * 0.3 * t + 1.5) +     # secondary component
    0.15 * np.random.randn(len(t))                # noise
)

# Save as CSV (time, value) — symview can load this directly
data = np.column_stack([t, signal])
np.savetxt("data/example_signal.csv", data, delimiter=",", header="time,value", comments="")

print("Generated data/example_signal.csv with", len(t), "samples")
```

Then run:

```bash
mkdir -p data
python generate_test_data.py
cargo run -p symworx-tui --bin symview
```

The file will appear in the Import tab.

#### Option 2: More realistic physiological data (using symworx-biosym)

If you have the Python package installed:

```python
import numpy as np
from symworx.biosym.physiology.ppg import generate_ppg_waveform

# Generate one PPG beat waveform
t0 = 0.0
duration = 1.0
fs = 250.0
beat_params = (0.2, 0.15, 0.6, 0.4, 0.35, 0.25)   # typical PPG shape parameters

times, values = generate_ppg_waveform(t0, duration, fs, beat_params)

# Repeat it a few times to make a longer recording
times = np.concatenate([times + i for i in range(30)])
values = np.tile(values, 30)

data = np.column_stack([times, values])
np.savetxt("data/example_ppg.csv", data, delimiter=",")
print("Generated data/example_ppg.csv")
```

#### Option 3: Using Rust + symworx-biosym (recommended for realistic data)

We now ship a small example that generates proper physiological signals using the real `symworx-biosym` crate:

```bash
# Generate realistic PPG + respiration data into data/
cargo run -p symworx-tui --example generate_biosym_demo
```

This will create:
- `data/demo_ppg_resting.csv`
- `data/demo_respiration.csv`

You can then load them directly in `symview`. This is the best way to quickly get realistic test data that exercises the dynamics features well.

---

## Tips

- Put generated files in a `data/` directory — they will be discovered automatically.
- Multi-column CSVs are supported with header awareness: the column picker will show actual names (e.g. "time", "ppg") when a header row is present.
- **To exit the TUI**: Press `q` at any time. `Esc` will cancel sub-modes (e.g. column selection, RQA params, spatial import) or stay if on Home.
- **Path selection (new)**: Starts on **Home** landing page. Choose:
  - 1 / Enter (on first): BioSym path (Import → Explore → Dynamics with real RQA)
  - 2: LoadSym path (training load / ACWR / NP/TSS/IF for .fit rides + nutrition)
  - 3: SpatialSym path (synthetic + import of match/game trajectory data inside Spatial tab)
- **Home always**: `Ctrl+H` or `0` returns to landing. Sub-views inside Spatial (g/i/v).
- **Tab switching**: `Ctrl+1` / `Ctrl+2` / `Ctrl+3` / `Ctrl+4` or arrows (context-aware; Home overrides).
- **RQA (Dynamics)**: With signal loaded, `c` opens param editor (m/tau/radius), Enter computes via symworx-dynamics.
- In the **Import** tab:
  - `Ctrl+G` → open the generate demo data menu (PPG / Respiration / Stride intervals)
  - `/` → enter filter mode
  - `c` → convert selected file to CSV
  - `Ctrl+R` (or `r` / F5) → refresh the file list
- `cargo run -p symworx-tui --example generate_biosym_demo` is still available as a standalone command.
