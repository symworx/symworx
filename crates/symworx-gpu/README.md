# symworx-gpu

Opt-in GPU kernels. The default build is a Rust recurrence-mask reference and does not look for a GPU. The `hip` feature compiles `kernels/recurrence.hip` with `hipcc`.

`HIP_PLATFORM=amd` uses the ROCm runtime (`libamdhip64`). `HIP_PLATFORM=nvidia` uses nvcc and `libcudart`. HIP does not emit one binary that runs on both. Build twice when you need both.

The kernel fills a packed Euclidean recurrence mask (squared distance, Theiler window). `symworx-dynamics` keeps `RecurrencePlot::from_trajectory` on the CPU. `try_from_trajectory_on_device` is the device call, behind the dynamics `gpu` / `hip` features. Line metrics (DET, LAM) stay on the CPU.

Use the device call for large masks, starting around a few thousand embedded points. Small plots should keep the CPU method. The dynamics wrapper refuses to expand a boolean matrix above `20_000²` cells.

`f64` only. Datacenter AMD and NVIDIA run that. Consumer RDNA runs it slowly. There is no `f32` kernel here.

## Build

CPU tests need only Rust:

```bash
cargo test -p symworx-gpu
```

Device tests need `hipcc` and a GPU in the same environment as `rustc`. They return immediately when the runtime reports no device.

```bash
HIP_PLATFORM=amd    cargo test -p symworx-gpu --features hip
HIP_PLATFORM=nvidia cargo test -p symworx-gpu --features hip

# dynamics wrappers
HIP_PLATFORM=amd cargo test -p symworx-dynamics --features hip
```

`SYMWORX_GPU_ARCH` overrides the offload arch (comma-separated `gfx…` or `sm_…` names). AMD defaults to `--offload-arch=native`. `SYMWORX_HIP_DEVICE` selects a device other than 0.

`dev-rust` has Rust and not HIP. `dev-amd` has HIP (`ROCM_PATH=/usr`, `HIP_PLATFORM=amd`) and not Rust. A device build needs both in one image. That image can also hold the CUDA toolkit. Each `cargo` invocation still sets one `HIP_PLATFORM`.
