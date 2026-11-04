// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Packed recurrence mask: CPU specification versus the device kernel.
//!
//! The device group is omitted when `probe()` is `None` (no `hip` feature, or
//! no usable device). Line metrics stay on the CPU and are not timed here.

use std::{
    hint::black_box,
    time::Duration,
};

use criterion::{
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use symworx_gpu::{
    Trajectory,
    probe,
    recurrence_mask,
    recurrence_mask_cpu,
};

const DIM: usize = 3;
const RADIUS: f64 = 0.5;
const THEILER: usize = 1;
const SIZES: [usize; 3] = [512, 2048, 4096];

fn points(n: usize) -> Vec<f64> {
    let mut coords = Vec::with_capacity(n * DIM);
    for i in 0..n {
        let t = i as f64 * 0.17;
        for axis in 0..DIM {
            coords.push((t + axis as f64).sin());
        }
    }
    coords
}

fn bench_recurrence_mask(c: &mut Criterion) {
    let device = probe();
    let mut group = c.benchmark_group("recurrence_mask");
    // One 4096² pass is the long sample. Ten of them is enough to compare.
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));

    for n in SIZES {
        let coords = points(n);
        let traj = Trajectory {
            points: &coords,
            n,
            dim: DIM,
        };
        group.throughput(Throughput::Elements((n * n) as u64));

        group.bench_with_input(BenchmarkId::new("cpu", n), &n, |b, _| {
            b.iter(|| black_box(recurrence_mask_cpu(traj, traj, RADIUS, THEILER).expect("cpu mask")));
        });

        if device.is_some() {
            group.bench_with_input(BenchmarkId::new("device", n), &n, |b, _| {
                b.iter(|| black_box(recurrence_mask(traj, traj, RADIUS, THEILER).expect("device mask")));
            });
        }
    }

    group.finish();
}

criterion_group!(benches, bench_recurrence_mask);
criterion_main!(benches);
