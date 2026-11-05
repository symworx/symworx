// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Sample entropy: CPU specification versus the device kernel.
//!
//! The device group is omitted when `probe()` is `None` (no `hip` feature, or
//! no usable device).

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
    probe,
    sample_entropy,
    sample_entropy_cpu,
};

const EMBED: usize = 2;
const RADIUS: f64 = 0.2;
const SIZES: [usize; 3] = [512, 2048, 4096];

fn series(n: usize) -> Vec<f64> {
    let mut values = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 * 0.17;
        values.push(t.sin());
    }
    values
}

fn bench_sample_entropy(c: &mut Criterion) {
    let device = probe();
    let mut group = c.benchmark_group("sample_entropy");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));

    for n in SIZES {
        let data = series(n);
        let pairs = (n - EMBED) as u64 * (n - EMBED - 1) as u64 / 2;
        group.throughput(Throughput::Elements(pairs));

        group.bench_with_input(BenchmarkId::new("cpu", n), &n, |b, _| {
            b.iter(|| black_box(sample_entropy_cpu(&data, EMBED, RADIUS).expect("cpu sampen")));
        });

        if device.is_some() {
            group.bench_with_input(BenchmarkId::new("device", n), &n, |b, _| {
                b.iter(|| black_box(sample_entropy(&data, EMBED, RADIUS).expect("device sampen")));
            });
        }
    }

    group.finish();
}

criterion_group!(benches, bench_sample_entropy);
criterion_main!(benches);
