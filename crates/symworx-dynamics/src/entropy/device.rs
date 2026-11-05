// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Device sample entropy.
//!
//! [`crate::entropy::sample_entropy`] stays the CPU function. This entry point
//! returns [`GpuError::Unavailable`] when `symworx-gpu` has no HIP device,
//! including a build without the `hip` feature. It does not fall back to the
//! CPU loop.

use symworx_gpu::GpuError;

/// Sample entropy from the device kernel.
///
/// The arguments and the `0.0` edge cases match [`crate::entropy::sample_entropy`].
/// A length that does not fit in `i32` is [`GpuError::InvalidInput`].
pub fn try_sample_entropy_on_device(data: &[f64], m: usize, r: f64) -> Result<f64, GpuError> {
    symworx_gpu::sample_entropy(data, m, r)
}

#[cfg(test)]
mod tests {
    use symworx_gpu::sample_entropy_cpu;

    use crate::entropy::sample_entropy;

    #[test]
    fn cpu_reference_matches_dynamics() {
        let data = [0.1, 0.4, 0.2, 0.8, 0.3, 0.7, 0.5, 0.25, 0.9, 0.15];
        for (m, r) in [(2usize, 0.2), (1, 0.05), (0, 0.3)] {
            let reference = sample_entropy_cpu(&data, m, r).unwrap();
            assert_eq!(reference, sample_entropy(&data, m, r));
        }
        assert_eq!(
            sample_entropy_cpu(&[5.0; 20], 2, 0.1).unwrap(),
            sample_entropy(&[5.0; 20], 2, 0.1)
        );
        assert_eq!(
            sample_entropy_cpu(&[1.0, 2.0, 3.0, 4.0], 2, 1e-12).unwrap(),
            sample_entropy(&[1.0, 2.0, 3.0, 4.0], 2, 1e-12)
        );
    }

    #[cfg(not(feature = "hip"))]
    #[test]
    fn device_entry_is_unavailable_without_hip() {
        let err = super::try_sample_entropy_on_device(&[0.0, 1.0, 0.2, 1.1, 0.1], 2, 0.5).unwrap_err();
        assert!(matches!(err, symworx_gpu::GpuError::Unavailable));
    }
}
