# symworx-gpu

Opt-in GPU kernels. The default build is a Rust recurrence-mask reference and a Rust sample-entropy reference. It does not look for a GPU. The `hip` feature compiles `kernels/recurrence.hip` and `kernels/sample_entropy.hip` into one shared library with `hipcc`.

`HIP_PLATFORM=amd` uses the ROCm runtime (`libamdhip64`). `HIP_PLATFORM=nvidia` uses nvcc and `libcudart`. HIP does not emit one binary that runs on both. Build twice when you need both.

The recurrence kernel fills a packed Euclidean recurrence mask (squared distance, Theiler window). `symworx-dynamics` keeps `RecurrencePlot::from_trajectory` on the CPU. `try_from_trajectory_on_device` is the device call, behind the dynamics `gpu` / `hip` features. Line metrics (DET, LAM) stay on the CPU.

The sample-entropy kernel counts Chebyshev matches on the raw series and returns `-ln(A/B)`. `sample_entropy` in dynamics stays on the CPU, as do multiscale entropy, the Dynamics tab, and the Python binding. `try_sample_entropy_on_device` is the device call. It uses the same `m`, `r`, and `0.0` edge cases as the CPU function, and it does not fall back to that loop.

Use a device call for long inputs, starting around a few thousand points. Small plots and short windows should keep the CPU method. The dynamics recurrence wrapper refuses to expand a boolean matrix above `20_000²` cells. Sample entropy does not build that matrix.

`f64` only. Datacenter AMD and NVIDIA run that. Consumer RDNA runs it slowly. There is no `f32` kernel here.

## Build

`detect_hardware()` reports two views. The PCI list is Linux sysfs (`/sys/bus/pci/devices`) and includes every display GPU plus AMD and NVIDIA accelerators. The HIP list is filled only when this crate was built with `hip` and that runtime can see a device. `compiled_for` is `Amd` or `Nvidia` for that build, and `None` otherwise. `compiled_target_matches_pci()` compares the build target with the PCI vendors.

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
