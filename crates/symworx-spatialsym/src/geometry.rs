// Copyright (c) 2026 Nathaniel T. Berry
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
        Self { x: -self.x, y: -self.y }
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
        if n == 0.0 { Self::zero() } else { self * (1.0 / n) }
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

fn cross_oab(o: Point2, a: Point2, b: Point2) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

/// Monotone-chain convex hull in counter-clockwise order (open: first ≠ last).
///
/// Duplicate / near-duplicate points are dropped. Fewer than three unique
/// points returns the unique set as-is.
pub fn convex_hull(points: &[Point2]) -> Vec<Point2> {
    let mut pts: Vec<Point2> = points.to_vec();
    pts.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    pts.dedup_by(|a, b| a.distance_squared(*b) < 1e-12);
    if pts.len() <= 2 {
        return pts;
    }

    let mut lower: Vec<Point2> = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross_oab(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<Point2> = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross_oab(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// Signed polygon area via the shoelace formula (absolute value is the geometric area).
pub fn polygon_area(pts: &[Point2]) -> f64 {
    if pts.len() < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        s += a.x * b.y - b.x * a.y;
    }
    s.abs() * 0.5
}

/// Compact triangles among the `k` nearest points to `focus`.
///
/// Triplets whose longest edge exceeds `max_edge_m`, or that are nearly
/// collinear, are dropped. Result is sorted by longest-edge (smallest first)
/// and capped at four triangles — enough for a local attacking shape without
/// drawing a full-team mesh.
pub fn local_triangles(points: &[Point2], focus: Point2, k: usize, max_edge_m: f64) -> Vec<[Point2; 3]> {
    if points.len() < 3 || max_edge_m <= 0.0 {
        return Vec::new();
    }
    let k = k.clamp(3, points.len());
    let mut idx: Vec<usize> = (0..points.len()).collect();
    idx.sort_by(|&a, &b| {
        points[a]
            .distance_squared(focus)
            .partial_cmp(&points[b].distance_squared(focus))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    idx.truncate(k);

    let mut ranked: Vec<(f64, [Point2; 3])> = Vec::new();
    for a in 0..idx.len() {
        for b in (a + 1)..idx.len() {
            for c in (b + 1)..idx.len() {
                let pa = points[idx[a]];
                let pb = points[idx[b]];
                let pc = points[idx[c]];
                let e = pa.distance(pb).max(pb.distance(pc)).max(pc.distance(pa));
                if e > max_edge_m {
                    continue;
                }
                let twice_area = (cross_oab(pa, pb, pc)).abs();
                if twice_area < 2.0 {
                    continue;
                }
                ranked.push((e, [pa, pb, pc]));
            }
        }
    }
    ranked.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));
    ranked.truncate(4);
    ranked.into_iter().map(|(_, t)| t).collect()
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

    #[test]
    fn convex_hull_of_square_drops_interior() {
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
            Point2::new(1.0, 1.0),
        ];
        let hull = convex_hull(&pts);
        assert_eq!(hull.len(), 4);
        assert!((polygon_area(&hull) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn local_triangles_keep_compact_triplet_near_focus() {
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(8.0, 0.0),
            Point2::new(4.0, 6.0),
            Point2::new(40.0, 40.0),
        ];
        let tris = local_triangles(&pts, Point2::new(4.0, 2.0), 4, 20.0);
        assert_eq!(tris.len(), 1);
        let far = local_triangles(&pts, Point2::new(4.0, 2.0), 4, 5.0);
        assert!(far.is_empty());
    }
}
