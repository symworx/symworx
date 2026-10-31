// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Compiles `kernels/recurrence.hip` when the `hip` feature is enabled.
//!
//! `hipcc` targets AMD or NVIDIA from `HIP_PLATFORM`. The kernel is linked
//! as a shared library so `hipcc` does the device link. Cargo does not run
//! this compiler for the default build.

use std::{
    env,
    path::{
        Path,
        PathBuf,
    },
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=kernels/recurrence.hip");
    println!("cargo:rerun-if-env-changed=HIP_PLATFORM");
    println!("cargo:rerun-if-env-changed=SYMWORX_GPU_ARCH");
    println!("cargo:rerun-if-env-changed=ROCM_PATH");
    println!("cargo:rerun-if-env-changed=CUDA_PATH");

    if env::var("CARGO_FEATURE_HIP").is_err() {
        return;
    }

    let platform = platform();
    if platform != "amd" && platform != "nvidia" {
        panic!("HIP_PLATFORM must be 'amd' or 'nvidia' (got {platform}). hipcc is the compiler for both.");
    }
    println!("cargo:rustc-env=SYMWORX_HIP_PLATFORM={platform}");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let lib_path = out_dir.join("libsymworx_hip.so");
    let hipcc = hipcc_path();

    let mut cmd = Command::new(&hipcc);
    cmd.env("HIP_PLATFORM", &platform)
        .args(["-O3", "-std=c++17", "-fPIC", "-shared"])
        .args(arch_flags(&platform))
        .args(fp_flags(&platform))
        .arg("kernels/recurrence.hip")
        .arg("-o")
        .arg(&lib_path);
    run(&mut cmd, &hipcc);

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=dylib=symworx_hip");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", out_dir.display());
    if platform == "nvidia" {
        let cuda = env::var("CUDA_PATH").unwrap_or_else(|_| "/usr/local/cuda".to_string());
        println!("cargo:rustc-link-search=native={cuda}/lib64");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{cuda}/lib64");
    }
}

fn platform() -> String {
    if let Ok(value) = env::var("HIP_PLATFORM") {
        return value.trim().to_ascii_lowercase();
    }
    let hipconfig = hipconfig_path();
    let output = Command::new(&hipconfig)
        .arg("--platform")
        .output()
        .unwrap_or_else(|err| {
            panic!(
                "failed to run {} --platform: {err}\nInstall ROCm (hipcc) or set HIP_PLATFORM=amd|nvidia.",
                hipconfig.display()
            )
        });
    if !output.status.success() {
        panic!(
            "hipconfig --platform failed ({})\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout).trim().to_ascii_lowercase()
}

fn arch_flags(platform: &str) -> Vec<String> {
    if let Ok(spec) = env::var("SYMWORX_GPU_ARCH") {
        let archs: Vec<&str> = spec.split(',').map(str::trim).filter(|arch| !arch.is_empty()).collect();
        if !archs.is_empty() {
            return archs
                .into_iter()
                .flat_map(|arch| ["--offload-arch".to_string(), arch.to_string()])
                .collect();
        }
    }
    if platform == "amd" {
        vec!["--offload-arch".to_string(), "native".to_string()]
    } else {
        Vec::new()
    }
}

fn fp_flags(platform: &str) -> Vec<&'static str> {
    // Keep the sum of squares in IEEE order so the mask matches the CPU loop
    // except for pairs that sit on the radius.
    if platform == "nvidia" {
        vec!["--fmad=false"]
    } else {
        vec!["-ffp-contract=off"]
    }
}

fn hipcc_path() -> PathBuf {
    find_tool("hipcc", &["/usr/bin/hipcc", "/opt/rocm/bin/hipcc"])
}

fn hipconfig_path() -> PathBuf {
    find_tool("hipconfig", &["/usr/bin/hipconfig", "/opt/rocm/bin/hipconfig"])
}

fn find_tool(name: &str, fallbacks: &[&str]) -> PathBuf {
    if let Some(paths) = env::var_os("PATH") {
        for dir in env::split_paths(&paths) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    for fallback in fallbacks {
        let path = PathBuf::from(fallback);
        if path.is_file() {
            return path;
        }
    }
    panic!(
        "could not find {name}. Install the ROCm HIP toolchain (hipcc) and put it on PATH.\n\
         AMD: HIP_PLATFORM=amd and ROCM_PATH (Fedora packages use /usr).\n\
         NVIDIA: HIP_PLATFORM=nvidia and CUDA_PATH (default /usr/local/cuda)."
    );
}

fn run(cmd: &mut Command, hipcc: &Path) {
    let output = cmd.output().unwrap_or_else(|err| {
        panic!("failed to spawn {}: {err}", hipcc.display());
    });
    if output.status.success() {
        return;
    }
    panic!(
        "hipcc failed ({})\ncommand: {cmd:?}\nstdout:\n{}\nstderr:\n{}\n\
         Need hipcc, HIP_PLATFORM=amd|nvidia, and ROCM_PATH or CUDA_PATH. \
         Override the GPU arch with SYMWORX_GPU_ARCH.",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
