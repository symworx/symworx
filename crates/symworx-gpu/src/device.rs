// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! HIP runtime calls. This module is the only `unsafe` in the crate.

#![allow(unsafe_code)]

use crate::{
    error::GpuError,
    recurrence::{
        DeviceInfo,
        RecurrenceMask,
        Trajectory,
        blank_mask,
        prepare,
    },
};

const PLATFORM: &str = env!("SYMWORX_HIP_PLATFORM");

#[allow(unsafe_code)]
mod ffi {
    unsafe extern "C" {
        fn symworx_hip_device_count(count: *mut i32) -> i32;
        fn symworx_hip_device_name(device: i32, name: *mut u8, len: usize) -> i32;
        fn symworx_recurrence_f64(
            left: *const f64,
            nx: i32,
            right: *const f64,
            ny: i32,
            dim: i32,
            radius2: f64,
            theiler: i32,
            same: i32,
            device: i32,
            host_bits: *mut u8,
            bits_len: i32,
        ) -> i32;
    }
}

pub fn probe_hip() -> Result<DeviceInfo, GpuError> {
    let count = device_count()?;
    let index = selected_device(count)?;
    let mut name_buf = [0u8; 256];
    // `name_buf` is a writable 256-byte buffer. The launcher writes a trailing NUL inside it.
    let status = unsafe { ffi::symworx_hip_device_name(index, name_buf.as_mut_ptr(), name_buf.len()) };
    if status != 0 {
        return Err(GpuError::Device {
            status,
            call: "symworx_hip_device_name",
        });
    }
    let end = name_buf.iter().position(|byte| *byte == 0).unwrap_or(name_buf.len());
    let name = String::from_utf8_lossy(&name_buf[..end]).into_owned();
    Ok(DeviceInfo {
        index,
        count,
        name,
        platform: PLATFORM,
    })
}

pub fn recurrence_mask_hip(
    left: Trajectory<'_>,
    right: Trajectory<'_>,
    radius: f64,
    theiler: usize,
) -> Result<RecurrenceMask, GpuError> {
    prepare(left, right, radius, theiler)?;
    if left.n == 0 || right.n == 0 {
        return Ok(RecurrenceMask {
            nrows: left.n,
            ncols: right.n,
            bits: Vec::new(),
        });
    }
    if radius < 0.0 {
        return blank_mask(left.n, right.n);
    }

    let device = probe_hip()?;
    let mut mask = blank_mask(left.n, right.n)?;
    let same = std::ptr::eq(left.points.as_ptr(), right.points.as_ptr()) && left.n == right.n && left.dim == right.dim;
    // Both slices stay borrowed for this synchronous launcher. `mask.bits` is exclusively
    // borrowed. When `same` is set the launcher reads `left` only.
    let status = unsafe {
        ffi::symworx_recurrence_f64(
            left.points.as_ptr(),
            left.n as i32,
            right.points.as_ptr(),
            right.n as i32,
            left.dim as i32,
            radius * radius,
            theiler as i32,
            i32::from(same),
            device.index,
            mask.bits.as_mut_ptr(),
            mask.bits.len() as i32,
        )
    };
    if status != 0 {
        return Err(GpuError::Device {
            status,
            call: "symworx_recurrence_f64",
        });
    }
    Ok(mask)
}

fn device_count() -> Result<i32, GpuError> {
    let mut count = 0;
    // `count` is a live i32 the launcher writes before returning.
    let status = unsafe { ffi::symworx_hip_device_count(&mut count) };
    if status != 0 || count <= 0 {
        return Err(GpuError::Unavailable);
    }
    Ok(count)
}

fn selected_device(count: i32) -> Result<i32, GpuError> {
    match std::env::var("SYMWORX_HIP_DEVICE") {
        Err(std::env::VarError::NotPresent) => Ok(0),
        Err(std::env::VarError::NotUnicode(_)) => Err(GpuError::InvalidInput("SYMWORX_HIP_DEVICE is not Unicode")),
        Ok(value) => {
            let index = value
                .parse::<i32>()
                .map_err(|_| GpuError::InvalidInput("SYMWORX_HIP_DEVICE is not an integer"))?;
            if index < 0 || index >= count {
                Err(GpuError::InvalidInput("SYMWORX_HIP_DEVICE is outside the device count"))
            } else {
                Ok(index)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::probe_hip;
    use crate::{
        recurrence::{
            Trajectory,
            recurrence_mask_cpu,
        },
        recurrence_mask,
    };

    #[test]
    fn hip_mask_matches_cpu_when_a_device_is_present() {
        if probe_hip().is_err() {
            return;
        }
        let points = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let left = Trajectory {
            points: &points,
            n: 3,
            dim: 2,
        };
        let cpu = recurrence_mask_cpu(left, left, 1.2, 0).unwrap();
        let gpu = recurrence_mask(left, left, 1.2, 0).unwrap();
        assert_eq!(cpu, gpu);

        let right_points = [0.0, 0.0, 5.0, 5.0, 0.2, 0.0];
        let right = Trajectory {
            points: &right_points,
            n: 3,
            dim: 2,
        };
        let cpu = recurrence_mask_cpu(left, right, 0.5, 1).unwrap();
        let gpu = recurrence_mask(left, right, 0.5, 1).unwrap();
        assert_eq!(cpu, gpu);
    }

    #[test]
    fn boundary_pair_may_disagree_only_on_the_radius() {
        if probe_hip().is_err() {
            return;
        }
        let origin = [0.0, 0.0, 0.0];
        let near = [0.1, -0.2, 0.3];
        let mut right_points = [0.0; 6];
        right_points[..3].copy_from_slice(&origin);
        right_points[3..].copy_from_slice(&near);
        let dist2 = near.iter().map(|value| value * value).sum::<f64>();
        let radius = dist2.sqrt();
        let left = Trajectory {
            points: &origin,
            n: 1,
            dim: 3,
        };
        let right = Trajectory {
            points: &right_points,
            n: 2,
            dim: 3,
        };
        let cpu = recurrence_mask_cpu(left, right, radius, 0).unwrap();
        let gpu = recurrence_mask(left, right, radius, 0).unwrap();
        if cpu.bit(0, 1) != gpu.bit(0, 1) {
            let gap = (dist2 - radius * radius).abs();
            assert!(gap <= dist2.abs() * 1e-12 + 1e-15);
        }
    }
}
