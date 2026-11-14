# symworx-npu

Scalar recurrence masks and RQA. The implementation is `f64` Rust in
this crate. A build runs on the CPU you compile for. It does not call
an AMD NPU, XDNA, or any other accelerator.

`recurrence_mask` fills a packed Euclidean mask (squared distance,
Theiler window). Bit 0 of each byte is the lowest column in that
byte. Pairs with `|i - j| <= theiler` stay unset. A negative radius is
an all-false mask. A non-finite radius is `NpuError::InvalidInput`. A
NaN coordinate is not recurrent.

`rqa` reads that mask. `RqaParams::min_line` defaults to 2. The stats
are recurrence rate (`rr`), determinism (`det`), laminarity (`lam`),
mean diagonal length, and diagonal-line entropy in nats.

`moving_average` is a centered window that shrinks at the ends. `Ema`
is causal. `Biquad::bandpass` is one 2nd-order Butterworth section,
direct form II transposed, with the same bilinear coefficients as
`symworx-signal`. `BandpassSpec` cascades that section.

`PeakFinder` takes a slice. A peak is a strict local maximum, then
height, neighbor threshold, prominence, and minimum distance. Distance
keeps the earlier peak. Non-finite samples are not peaks.

`detect_gait_peaks` uses a 0.5 s floor, prominence 0.08, and height
0.0, with no bandpass. `detect_gait_peaks_with_quality` adds the 0.2–6
Hz, two-section band and the reference, moderate, and poor thresholds
from `symworx-biosym`. `detect_ecg_r_peaks` is a 5–15 Hz band plus a
0.2 s refractory gap. It expects the R wave to be the tall narrow
peak. It does not run a derivative-energy detector or pick P and T
waves.

`repair_event_window` matches `symworx-signal`. Each interval is the
gap that closes on an event. Its probability is the worse of two
leave-one-out fits, `exp(-z² / 8)`, on the interval and on that
event's prominence. The window is admitted when the events span at
least 180 s and the mean probability is greater than 0.8. Intervals
below 0.6 are filled in place from the nearest kept intervals. A
malformed series is `NpuError::InvalidInput`. A scored series that
fails a gate is `Ok(None)`.

`rolling_event_measures` runs that repair on a grid. `MEASURE_STEP_SEC`
is 30 s and `MEASURE_STEP_FAST_SEC` is 15 s. Each sample looks back
180 s. An admitted sample reports RMSSD, CV, and sample entropy
(`m = 2`, tolerance `0.2 * SD`) of the repaired intervals. The same
sample carries the measured intervals and probabilities that close
inside that 15 s or 30 s step, plus their mean and standard deviation.
A grid time that is not admitted has `measures: None`.

`cross_sample_entropy` compares every template in one series with every
template in the other, including the pair that shares an index. The same
`r` makes the two call orders equal.

`cross_recurrence_mask` embeds both series the way host `edim` does.
A Theiler window of 0 leaves the simultaneous pair set. `recurrence_mask`
still clears `|i - j| == 0`. `crqa` reads that mask; its diagonal entropy
is in nats.

`joint_coupling` repairs both sides over the prior 180 s, drops oldest
intervals from the longer side, then calls those two with `m = 2`,
`tau = 1`, Theiler 0, and `r = 0.2 * SD` of the trimmed series pooled
together. `rolling_joint_coupling` uses the 15 s or 30 s grid. A side
that misses the 180 s gate has `coupling: None`. Fewer than two intervals
after the trim does too.

`dfa` is the scalar copy of `symworx-dynamics` detrended fluctuation
analysis. The default windows are boxes 4–16 and 16–64. The second
window needs 256 samples unless `always_alpha2` is set, and only when
its boxes fit. Heart rate and stride rate are not inputs. The log and
square root go through `libm`. This crate does not launch that
calculation on an accelerator.

`symworx-gpu` is the device path. Its `hip` feature compiles
`kernels/recurrence.hip` and `kernels/dfa.hip`. Line metrics and the
DFA slope stay on the CPU. This crate computes the mask, the line
metrics, and DFA in Rust.


```bash
cargo test -p symworx-npu
```

The default `std` feature is the host build. A Raspberry Pi Zero
(ARMv6, Raspberry Pi OS) uses that build:

```bash
cargo check -p symworx-npu --target arm-unknown-linux-gnueabihf
```

A Pi Zero 2 W is `aarch64-unknown-linux-gnu`. `--no-default-features`
is `no_std` plus `alloc`, with no `thiserror` and no `std`. `sin`,
`cos`, `ln`, `round`, `exp`, and `sqrt` go through the `libm` crate on
every build. The Arduino Nano 33 BLE (nRF52840) uses that build. Its
FPU is single precision, so these `f64` routines are soft-float on
that chip.

```bash
cargo check -p symworx-npu --target thumbv7em-none-eabihf --no-default-features
```

The board firmware has to provide the global allocator. This crate
does not.

```rust
use symworx_npu::{RqaParams, Trajectory, recurrence_mask, rqa};

let points = [0.0_f64, 1.0, 0.0, 1.0];
let traj = Trajectory {
    points: &points,
    n: 4,
    dim: 1,
};
let mask = recurrence_mask(traj, traj, 0.5, 1).unwrap();
let stats = rqa(&mask, RqaParams::default());
assert!(stats.rr > 0.0);
```
