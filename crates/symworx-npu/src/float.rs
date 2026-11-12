// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! `f64` operations that `core` does not provide.
//!
//! `libm` is pure Rust. The host tests and the Nano 33 BLE build call the
//! same functions. The bandpass golden check still matches the host section
//! within 1e-12.

#[inline]
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

#[inline]
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

#[inline]
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

#[inline]
pub fn round(x: f64) -> f64 {
    libm::round(x)
}
