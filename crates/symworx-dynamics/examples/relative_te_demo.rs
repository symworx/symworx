// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Multi-user relative discretization + discrete transfer entropy.
//!
//! Synthetic windowed "RMSSD" and sleep-stage labels. Each user has their
//! own raw offset/scale; bins are **relative** (high-for-this-user) from
//! pooled k-means cuts on the unit interval. Sleep stays as given labels.
//!
//! TE is computed **per night** (do not concatenate users or nights).
//!
//! ```bash
//! cargo run -p symworx-dynamics --example relative_te_demo
//! ```

use symworx_dynamics::{
    TeConfig,
    transfer_entropy_discrete,
};
use symworx_stats::{
    KMeansConfig,
    RelativeKMeansDiscretizer,
    SplitConfig,
    grouped_train_test_split,
};

const N_USERS: usize = 40;
const N_NIGHTS: usize = 3;
const N_WIN: usize = 80;
const N_BINS: usize = 3;

fn main() {
    println!("=== relative k-means discretize + discrete TE ===\n");

    let nights = generate_nights(2026);
    let (values, group_ids) = flatten_hrv(&nights);

    let split = grouped_train_test_split(
        &group_ids,
        &SplitConfig {
            test_ratio: 0.3,
            n_train_folds: None,
            shuffle: true,
            seed: 11,
        },
    )
    .expect("40 users is enough for a 70/30 grouped split");

    let train_users = unique_from_rows(&group_ids, &split.train_idx);
    let test_users = unique_from_rows(&group_ids, &split.test_idx);
    println!("1) Users: {N_USERS}  nights/user: {N_NIGHTS}  windows/night: {N_WIN}");
    println!(
        "   grouped split: {} train users, {} test users",
        train_users.len(),
        test_users.len()
    );

    let train_values: Vec<f64> = split.train_idx.iter().map(|&i| values[i]).collect();
    let train_groups: Vec<usize> = split.train_idx.iter().map(|&i| group_ids[i]).collect();

    let disc = RelativeKMeansDiscretizer::fit(
        &train_values,
        &train_groups,
        N_BINS,
        &KMeansConfig {
            k: N_BINS,
            seed: 3,
            ..KMeansConfig::default()
        },
    )
    .expect("fit relative k-means discretizer");

    println!("\n2) Shared unit-interval cuts (n_bins = {N_BINS}): {:?}", disc.cuts);
    println!("   centroids: {:?}", disc.centroids);

    let cfg = TeConfig {
        k: 1,
        l: 1,
        tau: 1,
        horizon: 1,
        bins: 4, // ignored on the discrete path
    };

    let (te_s2h_train, te_h2s_train) = mean_te_for_users(&nights, &train_users, &disc, true, &cfg);
    let (te_s2h_test, te_h2s_test) = mean_te_for_users(&nights, &test_users, &disc, false, &cfg);

    println!("\n3) Mean per-night TE (nats)");
    println!("   train  TE(sleep → HRV bins) = {te_s2h_train:.4}   TE(HRV bins → sleep) = {te_h2s_train:.4}");
    println!("   test   TE(sleep → HRV bins) = {te_s2h_test:.4}   TE(HRV bins → sleep) = {te_h2s_test:.4}");
    println!("   (sleep drives synthetic HRV, so sleep → HRV should be larger)");

    let u0: Vec<f64> = nights
        .iter()
        .filter(|n| n.user == 0)
        .flat_map(|n| n.hrv.iter().copied())
        .collect();
    let bins0 = disc.transform_new_group(&u0);
    let shifted: Vec<f64> = u0.iter().map(|v| v + 250.0).collect();
    let bins_shift = disc.transform_new_group(&shifted);
    println!(
        "\n4) Relative invariance: user 0 bins unchanged after +250 raw offset? {}",
        bins0 == bins_shift
    );
}

struct Night {
    user: usize,
    hrv: Vec<f64>,
    sleep: Vec<u8>,
}

fn generate_nights(seed: u64) -> Vec<Night> {
    let mut s = seed;
    let mut next = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((s >> 33) as f64) / (u32::MAX as f64)
    };

    let mut nights = Vec::with_capacity(N_USERS * N_NIGHTS);
    for user in 0..N_USERS {
        let offset = 20.0 + 40.0 * next();
        let scale = 8.0 + 20.0 * next();
        for _night in 0..N_NIGHTS {
            let mut sleep = Vec::with_capacity(N_WIN);
            let mut hrv = Vec::with_capacity(N_WIN);
            let mut stage = 0u8;
            let mut dwell = 0usize;
            let mut latent = 0.4;
            for _t in 0..N_WIN {
                if dwell == 0 {
                    stage = (stage + 1) % 5;
                    dwell = 8 + (next() * 6.0) as usize;
                }
                dwell = dwell.saturating_sub(1);
                // High relative HRV more often after REM/wake-like stages (3, 4).
                let drive = if stage >= 3 { 0.85 } else { 0.25 };
                latent = 0.65 * latent + 0.35 * drive + 0.05 * (next() - 0.5);
                let rel = latent.clamp(0.0, 1.0);
                sleep.push(stage);
                hrv.push(offset + scale * rel + 0.4 * (next() - 0.5));
            }
            nights.push(Night { user, hrv, sleep });
        }
    }
    nights
}

fn flatten_hrv(nights: &[Night]) -> (Vec<f64>, Vec<usize>) {
    let mut values = Vec::new();
    let mut groups = Vec::new();
    for n in nights {
        values.extend_from_slice(&n.hrv);
        groups.extend(std::iter::repeat_n(n.user, n.hrv.len()));
    }
    (values, groups)
}

fn unique_from_rows(group_ids: &[usize], idx: &[usize]) -> Vec<usize> {
    let mut seen = Vec::new();
    for &i in idx {
        let g = group_ids[i];
        if !seen.contains(&g) {
            seen.push(g);
        }
    }
    seen
}

fn mean_te_for_users(
    nights: &[Night],
    users: &[usize],
    disc: &RelativeKMeansDiscretizer,
    train: bool,
    cfg: &TeConfig,
) -> (f64, f64) {
    let mut s2h = 0.0;
    let mut h2s = 0.0;
    let mut n = 0.0;
    for &user in users {
        let user_nights: Vec<&Night> = nights.iter().filter(|rec| rec.user == user).collect();
        let hrv: Vec<f64> = user_nights.iter().flat_map(|rec| rec.hrv.iter().copied()).collect();
        let bins = if train {
            disc.transform(&hrv, &vec![user; hrv.len()])
                .expect("train user is fitted")
        } else {
            disc.transform_new_group(&hrv)
        };
        let mut offset = 0usize;
        for rec in &user_nights {
            let end = offset + rec.hrv.len();
            let hb = &bins[offset..end];
            s2h += transfer_entropy_discrete(&rec.sleep, hb, cfg);
            h2s += transfer_entropy_discrete(hb, &rec.sleep, cfg);
            n += 1.0;
            offset = end;
        }
    }
    (s2h / n, h2s / n)
}
