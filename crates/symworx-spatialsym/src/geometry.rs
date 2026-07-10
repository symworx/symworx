// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Core 2D geometry primitives: points and vectors.
//!
//! All units are meters (positions) and meters/second (velocities) unless documented otherwise.
//! These types are deliberately small, `Copy`, and allocation-free.

use std::ops::{
    Add,
    AddAssign,
    Mul,
    Neg,
    Sub,
    SubAssign,
};

/// A 2D point (position in meters).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point2 {
    /// X coordinate (meters)
    pub x: f64,
    /// Y coordinate (meters)
    pub y: f64,
}

/// A 2D vector (displacement or velocity).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec2 {
    /// X component (meters or m/s)
    pub x: f64,
    /// Y component (meters or m/s)
    pub y: f64,
}

// Constructors
impl Point2 {
    /// Create a new point.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// The origin.
    pub const fn origin() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

impl Vec2 {
    /// Create a new vector.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// The zero vector.
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

// Conversions
impl From<[f64; 2]> for Point2 {
    fn from([x, y]: [f64; 2]) -> Self {
        Self { x, y }
    }
}

impl From<[f64; 2]> for Vec2 {
    fn from([x, y]: [f64; 2]) -> Self {
        Self { x, y }
    }
}

impl From<Point2> for [f64; 2] {
    fn from(p: Point2) -> Self {
        [p.x, p.y]
    }
}

impl From<Vec2> for [f64; 2] {
    fn from(v: Vec2) -> Self {
        [v.x, v.y]
    }
}

// Basic ops for Vec2 (primary arithmetic type)
impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<f64> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

// Point + Vec and Point - Point → Vec
impl Add<Vec2> for Point2 {
    type Output = Self;
    fn add(self, rhs: Vec2) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Point2 {
    type Output = Vec2;
    fn sub(self, rhs: Self) -> Self::Output {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Sub<Vec2> for Point2 {
    type Output = Self;
    fn sub(self, rhs: Vec2) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

// Vec2 utility methods
impl Vec2 {
    /// Dot product.
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// Squared Euclidean norm (avoids sqrt when possible).
    pub fn norm_squared(self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    /// Euclidean norm (length).
    pub fn norm(self) -> f64 {
        self.norm_squared().sqrt()
    }

    /// Returns a unit vector in the same direction (NaN if zero length).
    pub fn normalize(self) -> Self {
        let n = self.norm();
        if n == 0.0 {
            Self::zero()
        } else {
            self * (1.0 / n)
        }
    }

    /// Bearing in radians using atan2(dy, dx), range (-pi, pi].
    /// 0 points along positive x; positive angles counterclockwise (standard math convention).
    pub fn bearing(self) -> f64 {
        self.y.atan2(self.x)
    }

    /// Angle between self and other in radians [0, pi].
    pub fn angle_to(self, other: Self) -> f64 {
        let cos = (self.dot(other) / (self.norm() * other.norm() + 1e-12)).clamp(-1.0, 1.0);
        cos.acos()
    }
}

// Point2 utilities
impl Point2 {
    /// Euclidean distance to another point.
    pub fn distance(self, other: Self) -> f64 {
        (self - other).norm()
    }

    /// Squared distance (cheaper).
    pub fn distance_squared(self, other: Self) -> f64 {
        (self - other).norm_squared()
    }
}

// Free functions for ergonomics. (Batched support can be added later without ndarray if needed.)
/// Euclidean distance between two points.
pub fn distance(p1: Point2, p2: Point2) -> f64 {
    p1.distance(p2)
}

/// Bearing (atan2) of a vector.
pub fn bearing(v: Vec2) -> f64 {
    v.bearing()
}

/// Bearing from one point to another (direction from `from` toward `to`).
pub fn bearing_between(from: Point2, to: Point2) -> f64 {
    (to - from).bearing()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_vec_basic_ops() {
        let p0 = Point2::origin();
        let p1 = Point2::new(3.0, 4.0);
        let v = p1 - p0;
        assert_eq!(v, Vec2::new(3.0, 4.0));
        assert!((v.norm() - 5.0).abs() < 1e-12);

        let v2 = Vec2::new(0.0, 1.0);
        assert!((v2.bearing() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }

    #[test]
    fn distance_and_normalize() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        assert!((distance(a, b) - 1.0).abs() < 1e-12);

        let v = Vec2::new(3.0, 0.0).normalize();
        assert!((v.x - 1.0).abs() < 1e-12);
        assert!(v.y.abs() < 1e-12);
    }

    #[test]
    fn point_add_vec() {
        let p = Point2::new(1.0, 2.0) + Vec2::new(0.5, -1.0);
        assert_eq!(p, Point2::new(1.5, 1.0));
    }
}
