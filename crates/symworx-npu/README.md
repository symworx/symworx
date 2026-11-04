# symworx-npu

Scalar recurrence masks and RQA. The implementation is `f64` Rust in this crate. A build runs on the CPU you compile for. It does not call an AMD NPU, XDNA, or any other accelerator.

`recurrence_mask` fills a packed Euclidean mask (squared distance, Theiler window). Bit 0 of each byte is the lowest column in that byte. Pairs with `|i - j| <= theiler` stay unset. A negative radius is an all-false mask. A non-finite radius is `NpuError::InvalidInput`. A NaN coordinate is not recurrent.

`rqa` reads that mask. `RqaParams::min_line` defaults to 2. The stats are recurrence rate (`rr`), determinism (`det`), laminarity (`lam`), mean diagonal length, and diagonal-line entropy in nats.

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
