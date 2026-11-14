// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Mean-centered cumulative profile for detrended fluctuation analysis.

use super::{
    DfaError,
    check_series,
};

/// Mean-centered cumulative profile `Y(k)`.
///
/// `Y(k)` is the running sum of `series - mean(series)`. The input is the
/// measured series. Passing a profile here integrates it again.
pub fn profile(series: &[f64]) -> Result<Vec<f64>, DfaError> {
    check_series(series)?;
    let mean = series.iter().sum::<f64>() / series.len() as f64;
    let mut acc = 0.0;
    let mut out = Vec::with_capacity(series.len());
    for &sample in series {
        acc += sample - mean;
        out.push(acc);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_rejects_an_empty_or_non_finite_series() {
        assert_eq!(profile(&[]), Err(DfaError::Empty));
        assert_eq!(profile(&[1.0, f64::NAN]), Err(DfaError::NonFinite));
        assert_eq!(profile(&[1.0, f64::INFINITY]), Err(DfaError::NonFinite));
    }

    #[test]
    fn profile_is_the_cumulative_sum_of_the_centered_series() {
        assert_eq!(profile(&[1.0, 2.0, 3.0]).unwrap(), vec![-1.0, -1.0, 0.0]);
        assert_eq!(profile(&[5.0, 5.0, 5.0, 5.0]).unwrap(), vec![0.0, 0.0, 0.0, 0.0]);
    }
}
