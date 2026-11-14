// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Device fluctuation for detrended fluctuation analysis.
//!
//! The profile, the scale windows, and the log-log slope stay on the CPU.
//! Order-1 `F(n)` is [`symworx_gpu::dfa_fluctuation`]. This returns
//! [`GpuError::Unavailable`] when that call has no HIP device, including a
//! build without the `hip` feature. It does not fall back to the CPU
//! fluctuation. Polynomial order above 1 is rejected here.

use symworx_gpu::GpuError;

use super::{
    DfaConfig,
    DfaError,
    DfaResult,
    check_series,
    fit_window,
    profile,
    windows::select_windows,
};

/// Detrended fluctuation analysis with order-1 `F(n)` on a HIP device.
///
/// Pass the measured series. This builds the profile and chooses the windows
/// with the same rules as [`super::dfa`], then asks the device for `F(n)`.
/// The slope is fit on the CPU. [`DfaConfig::order`] must be 1.
pub fn dfa_on_device(series: &[f64], config: &DfaConfig) -> Result<DfaResult, GpuError> {
    check_series(series).map_err(gpu_input)?;
    match config.order {
        0 => return Err(GpuError::InvalidInput("polynomial order was zero")),
        1 => {}
        _ => return Err(GpuError::InvalidInput("DFA order above 1 stays on the CPU")),
    }
    let slots = select_windows(config, series.len()).map_err(gpu_input)?;
    let integrated = profile(series).map_err(gpu_input)?;

    let mut scales = Vec::new();
    for window in slots.iter().flatten() {
        scales.extend(window.iter().copied());
    }
    scales.sort_unstable();
    scales.dedup();

    let fluctuation_curve = symworx_gpu::dfa_fluctuation(&integrated, &scales)?;
    if fluctuation_curve.len() != scales.len() {
        return Err(GpuError::InvalidInput(
            "device fluctuation length does not match the scales",
        ));
    }

    let mut alphas = Vec::with_capacity(slots.len());
    for slot in slots {
        let fit = match slot {
            Some(window) => Some(fit_window(&scales, &fluctuation_curve, &window).map_err(gpu_input)?),
            None => None,
        };
        alphas.push(fit);
    }
    Ok(DfaResult {
        scales,
        fluctuation: fluctuation_curve,
        alphas,
    })
}

fn gpu_input(err: DfaError) -> GpuError {
    GpuError::InvalidInput(match err {
        DfaError::Empty => "series is empty",
        DfaError::NonFinite => "series has a non-finite sample",
        DfaError::InvalidOrder => "polynomial order was zero",
        DfaError::InvalidScale => "scale does not fit the series",
        DfaError::TooFewScales => "a window needs at least two scales",
        DfaError::LengthMismatch => "scales and fluctuation differ in length",
        DfaError::NonPositiveFluctuation => "fluctuation is not positive",
        DfaError::SingularFit => "log-log fit is singular",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn short_config(alpha_scales: Vec<Vec<usize>>) -> DfaConfig {
        DfaConfig {
            order: 1,
            alpha_scales,
            min_repeats: 1,
            always_alpha2: false,
        }
    }

    #[test]
    fn device_dfa_rejects_input_before_the_launch() {
        let err = dfa_on_device(&[], &DfaConfig::default()).unwrap_err();
        assert!(matches!(err, GpuError::InvalidInput("series is empty")));

        let err = dfa_on_device(&[1.0, f64::NAN, 0.0, 1.0], &short_config(vec![vec![3, 4]])).unwrap_err();
        assert!(matches!(err, GpuError::InvalidInput("series has a non-finite sample")));

        let mut config = short_config(vec![vec![4, 6]]);
        config.order = 0;
        let err = dfa_on_device(&[0.0, 1.0, 0.0, 1.0, 0.0, 1.0], &config).unwrap_err();
        assert!(matches!(err, GpuError::InvalidInput("polynomial order was zero")));

        config.order = 2;
        let err = dfa_on_device(&[0.0, 1.0, 0.0, 1.0, 0.0, 1.0], &config).unwrap_err();
        assert!(matches!(
            err,
            GpuError::InvalidInput("DFA order above 1 stays on the CPU")
        ));

        let err = dfa_on_device(&[0.0; 8], &DfaConfig::default()).unwrap_err();
        assert!(matches!(err, GpuError::InvalidInput("scale does not fit the series")));
    }

    #[cfg(not(feature = "hip"))]
    #[test]
    fn device_dfa_reports_unavailable_without_hip() {
        let series = [0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0];
        let err = dfa_on_device(&series, &short_config(vec![vec![3, 4]])).unwrap_err();
        assert!(matches!(err, GpuError::Unavailable));
    }

    #[cfg(feature = "hip")]
    #[test]
    fn device_dfa_matches_cpu_when_a_device_is_present() {
        fn assert_close(got: f64, expect: f64) {
            assert!((got - expect).abs() <= 1e-12, "{got} != {expect}");
        }

        if symworx_gpu::probe().is_none() {
            return;
        }
        let series = [0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0];
        let config = short_config(vec![vec![3, 4], vec![4, 8]]);
        let cpu = super::super::dfa(&series, &config).unwrap();
        let device = dfa_on_device(&series, &config).unwrap();
        assert_eq!(device.scales, cpu.scales);
        assert_eq!(device.alphas.len(), cpu.alphas.len());
        for (got, expect) in device.fluctuation.iter().zip(cpu.fluctuation.iter()) {
            assert_close(*got, *expect);
        }
        for (got, expect) in device.alphas.iter().zip(cpu.alphas.iter()) {
            match (got, expect) {
                (Some(got), Some(expect)) => {
                    assert_eq!(got.scales, expect.scales);
                    assert_close(got.alpha, expect.alpha);
                    assert_close(got.intercept, expect.intercept);
                }
                (None, None) => {}
                _ => panic!("window presence differs"),
            }
        }

        let config = DfaConfig {
            always_alpha2: true,
            ..DfaConfig::default()
        };
        let series = vec![0.25; 128];
        let cpu = super::super::dfa(&series, &config).unwrap_err();
        let device = dfa_on_device(&series, &config).unwrap_err();
        assert!(matches!(cpu, super::super::DfaError::NonPositiveFluctuation));
        assert!(matches!(device, GpuError::InvalidInput("fluctuation is not positive")));
    }
}
