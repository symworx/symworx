// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Order-1 fluctuation `F(n)` for detrended fluctuation analysis.
//!
//! The host chooses the box lengths. This module does not fit a log-log slope
//! and does not read heart rate or stride rate. [`dfa_fluctuation_cpu`] is the
//! specification: Kantelhardt tiles the profile from both ends and removes a
//! line in each box. [`dfa_fluctuation`] launches that residual sum on HIP and
//! returns [`GpuError::Unavailable`] without the `hip` feature or a device.
//! It does not fall back to the CPU function. Polynomial order above 1 stays
//! in `symworx-dynamics`.

use crate::error::GpuError;

/// Order-1 fluctuation `F(n)` at each box length.
///
/// `profile` is the integrated series. Each scale is one result, in the order
/// given. A scale must be at least 3 and no longer than the profile.
/// Without `hip`, or when the runtime has no device, this returns
/// [`GpuError::Unavailable`] and does not run [`dfa_fluctuation_cpu`].
pub fn dfa_fluctuation(profile: &[f64], scales: &[usize]) -> Result<Vec<f64>, GpuError> {
    #[cfg(not(feature = "hip"))]
    {
        let _ = (profile, scales);
        Err(GpuError::Unavailable)
    }
    #[cfg(feature = "hip")]
    {
        fluctuation_hip(profile, scales)
    }
}

/// CPU order-1 fluctuation. This is the specification the device kernel matches.
///
/// Boxes of length `scale` tile `profile` from the start and again from the
/// end. `F` is the root mean square of the residuals about a least-squares
/// line, across both passes.
pub fn dfa_fluctuation_cpu(profile: &[f64], scales: &[usize]) -> Result<Vec<f64>, GpuError> {
    check_profile(profile, scales)?;
    let mut out = Vec::with_capacity(scales.len());
    for &scale in scales {
        out.push(fluctuation_one(profile, scale)?);
    }
    Ok(out)
}

fn check_profile(profile: &[f64], scales: &[usize]) -> Result<(), GpuError> {
    if profile.is_empty() {
        return Err(GpuError::InvalidInput("profile is empty"));
    }
    if profile.iter().any(|sample| !sample.is_finite()) {
        return Err(GpuError::InvalidInput("profile has a non-finite sample"));
    }
    if scales.iter().any(|&scale| scale < 3 || scale > profile.len()) {
        return Err(GpuError::InvalidInput("scale does not fit the profile"));
    }
    Ok(())
}

fn fluctuation_one(profile: &[f64], scale: usize) -> Result<f64, GpuError> {
    let n_seg = profile.len() / scale;
    let mut sum_sq = 0.0;
    for segment in profile.chunks_exact(scale) {
        sum_sq += linear_residual_sum_sq(segment);
    }
    for seg in 0..n_seg {
        let start = profile.len() - (seg + 1) * scale;
        sum_sq += linear_residual_sum_sq(&profile[start..start + scale]);
    }
    let denom = denominator(n_seg, scale)?;
    Ok((sum_sq / denom).sqrt())
}

/// `2 * n_seg * scale` as `f64`. Rejects a product that does not fit in `usize`.
fn denominator(n_seg: usize, scale: usize) -> Result<f64, GpuError> {
    let denom = 2usize
        .checked_mul(n_seg)
        .and_then(|value| value.checked_mul(scale))
        .ok_or(GpuError::InvalidInput("fluctuation denominator overflows"))?;
    Ok(denom as f64)
}

/// Sum of squared residuals about the least-squares line.
///
/// The time index is `0 .. n-1`. `sum (t - mean_t)^2 = n (n^2 - 1) / 12`.
fn linear_residual_sum_sq(segment: &[f64]) -> f64 {
    let n = segment.len() as f64;
    let mean_t = (n - 1.0) / 2.0;
    let mean_y = segment.iter().sum::<f64>() / n;
    let mut cov = 0.0;
    for (i, &sample) in segment.iter().enumerate() {
        cov += (i as f64 - mean_t) * (sample - mean_y);
    }
    let sum_sq_t = n * (n * n - 1.0) / 12.0;
    let slope = cov / sum_sq_t;
    let intercept = mean_y - slope * mean_t;
    let mut sum_sq = 0.0;
    for (i, &sample) in segment.iter().enumerate() {
        let residual = sample - (intercept + slope * i as f64);
        sum_sq += residual * residual;
    }
    sum_sq
}

#[cfg(feature = "hip")]
fn fluctuation_hip(profile: &[f64], scales: &[usize]) -> Result<Vec<f64>, GpuError> {
    check_profile(profile, scales)?;
    if scales.is_empty() {
        return Ok(Vec::new());
    }
    if profile.len() > i32::MAX as usize {
        return Err(GpuError::InvalidInput("profile length does not fit in i32"));
    }

    let mut starts = Vec::new();
    let mut box_scales = Vec::new();
    let mut counts = Vec::with_capacity(scales.len());
    for &scale in scales {
        let n_seg = profile.len() / scale;
        let count = n_seg
            .checked_mul(2)
            .ok_or(GpuError::InvalidInput("box count overflows"))?;
        counts.push(count);
        for seg in 0..n_seg {
            starts.push((seg * scale) as i32);
            box_scales.push(scale as i32);
        }
        for seg in 0..n_seg {
            let start = profile.len() - (seg + 1) * scale;
            starts.push(start as i32);
            box_scales.push(scale as i32);
        }
    }
    if starts.len() > i32::MAX as usize {
        return Err(GpuError::InvalidInput("box count does not fit in i32"));
    }

    let ss = crate::device::dfa_linear_ss_hip(profile, &starts, &box_scales)?;
    if ss.len() != starts.len() {
        return Err(GpuError::InvalidInput(
            "device residual length does not match the boxes",
        ));
    }

    let mut out = Vec::with_capacity(scales.len());
    let mut offset = 0;
    for (index, &scale) in scales.iter().enumerate() {
        let count = counts[index];
        let mut sum_sq = 0.0;
        for value in &ss[offset..offset + count] {
            sum_sq += value;
        }
        offset += count;
        let n_seg = profile.len() / scale;
        let denom = denominator(n_seg, scale)?;
        out.push((sum_sq / denom).sqrt());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(got: f64, expect: f64) {
        assert!((got - expect).abs() <= 1e-12, "{got} != {expect}");
    }

    #[test]
    fn cpu_fluctuation_matches_the_hand_residuals() {
        let line = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let zeros = dfa_fluctuation_cpu(&line, &[3, 6]).unwrap();
        assert_close(zeros[0], 0.0);
        assert_close(zeros[1], 0.0);

        let step = dfa_fluctuation_cpu(&[0.0, 0.0, 1.0, 1.0], &[4]).unwrap();
        assert_close(step[0], (0.05_f64).sqrt());

        // Forward box [0, 0, 0] has no residual. The tail [0, 1, 1] does.
        // Scale 4 on the same profile is one forward box and one backward box.
        let tail = dfa_fluctuation_cpu(&[0.0, 0.0, 0.0, 1.0, 1.0], &[3, 4]).unwrap();
        assert_close(tail[0], 1.0 / 6.0);
        assert_close(tail[1], 0.25);
    }

    #[test]
    fn cpu_fluctuation_keeps_the_requested_scale_order() {
        let profile = [0.0, 0.0, 0.0, 1.0, 1.0];
        let forward = dfa_fluctuation_cpu(&profile, &[3, 4]).unwrap();
        let reversed = dfa_fluctuation_cpu(&profile, &[4, 3]).unwrap();
        assert_close(reversed[0], forward[1]);
        assert_close(reversed[1], forward[0]);
    }

    #[test]
    fn cpu_fluctuation_rejects_a_bad_profile_or_scale() {
        assert!(matches!(
            dfa_fluctuation_cpu(&[], &[3]),
            Err(GpuError::InvalidInput("profile is empty"))
        ));
        assert!(matches!(
            dfa_fluctuation_cpu(&[0.0, f64::NAN, 1.0, 1.0], &[3]),
            Err(GpuError::InvalidInput("profile has a non-finite sample"))
        ));
        assert!(matches!(
            dfa_fluctuation_cpu(&[0.0, 1.0, 2.0, 3.0], &[2]),
            Err(GpuError::InvalidInput("scale does not fit the profile"))
        ));
        assert!(matches!(
            dfa_fluctuation_cpu(&[0.0, 1.0, 2.0, 3.0], &[5]),
            Err(GpuError::InvalidInput("scale does not fit the profile"))
        ));
    }

    #[cfg(not(feature = "hip"))]
    #[test]
    fn device_fluctuation_is_unavailable_without_hip() {
        let err = dfa_fluctuation(&[0.0, 1.0, 2.0, 3.0], &[3]).unwrap_err();
        assert!(matches!(err, GpuError::Unavailable));
    }

    #[cfg(feature = "hip")]
    #[test]
    fn device_fluctuation_matches_cpu_when_a_device_is_present() {
        if crate::probe().is_none() {
            return;
        }
        let profile = [0.0, 0.0, 0.0, 1.0, 1.0];
        let scales = [4, 3];
        let cpu = dfa_fluctuation_cpu(&profile, &scales).unwrap();
        let device = dfa_fluctuation(&profile, &scales).unwrap();
        for (got, expect) in device.iter().zip(cpu.iter()) {
            assert_close(*got, *expect);
        }

        let mut state = 0x1234_5678_9abc_u64;
        let mut long = Vec::with_capacity(256);
        for _ in 0..256 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let unit = (state >> 11) as f64 / (1u64 << 53) as f64;
            long.push(unit * 2.0 - 1.0);
        }
        let scales: Vec<usize> = (4..=64).collect();
        let cpu = dfa_fluctuation_cpu(&long, &scales).unwrap();
        let device = dfa_fluctuation(&long, &scales).unwrap();
        assert_eq!(device.len(), cpu.len());
        for (got, expect) in device.iter().zip(cpu.iter()) {
            let tol = expect.abs() * 1e-9 + 1e-12;
            assert!((got - expect).abs() <= tol, "{got} != {expect}");
        }
    }
}
