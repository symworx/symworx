# symworx-npu

Scalar recurrence masks and RQA. The implementation is `f64` Rust in this crate. A build runs on the CPU you compile for. It does not call an AMD NPU, XDNA, or any other accelerator.

`recurrence_mask` fills a packed Euclidean mask (squared distance, Theiler window). Bit 0 of each byte is the lowest column in that byte. Pairs with `|i - j| <= theiler` stay unset. A negative radius is an all-false mask. A non-finite radius is `NpuError::InvalidInput`. A NaN coordinate is not recurrent.

`rqa` reads that mask. `RqaParams::min_line` defaults to 2. The stats are recurrence rate (`rr`), determinism (`det`), laminarity (`lam`), mean diagonal length, and diagonal-line entropy in nats.

`moving_average` is a centered window that shrinks at the ends. `Ema` is causal. `Biquad::bandpass` is one 2nd-order Butterworth section, direct form II transposed, with the same bilinear coefficients as `symworx-signal`. `BandpassSpec` cascades that section.

`PeakFinder` takes a slice. A peak is a strict local maximum, then height, neighbor threshold, prominence, and minimum distance. Distance keeps the earlier peak. Non-finite samples are not peaks.

`detect_gait_peaks` uses a 0.5 s floor, prominence 0.08, and height 0.0, with no bandpass. `detect_gait_peaks_with_quality` adds the 0.2–6 Hz, two-section band and the reference, moderate, and poor thresholds from `symworx-biosym`. `detect_ecg_r_peaks` is a 5–15 Hz band plus a 0.2 s refractory gap. It expects the R wave to be the tall narrow peak. It does not run a derivative-energy detector or pick P and T waves.

`symworx-gpu` is the device path. Its `hip` feature compiles `kernels/recurrence.hip`. Line metrics there stay on the CPU. This crate computes the mask and the line metrics in Rust.


```bash
cargo test -p symworx-npu
```

The `std` feature is on by default and unused. `lib.rs` has no `#![no_std]`, and `thiserror` keeps its default `std` feature. `--no-default-features` still builds a host crate.

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
