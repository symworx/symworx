// Copyright (c) 2026 SymWorx. All rights reserved.
// Licensed under the Mozilla Public License, Version 2.0.

//! Trajectory containers (time + 2D positions).
//!
//! Simple owned vectors per agent. Efficient for typical team sizes; batched support can be added later using pure Rust if needed.

use crate::geometry::{
    Point2,
    Vec2,
};

/// A simple time-stamped trajectory for a single agent/entity.
#[derive(Clone, Debug, PartialEq)]
pub struct Trajectory {
    /// Monotonic time stamps (seconds).
    pub times: Vec<f64>,
    /// Corresponding 2D positions (meters).
    pub positions: Vec<Point2>,
}

impl Trajectory {
    /// Construct a trajectory. Lengths of `times` and `positions` must match.
    pub fn new(times: Vec<f64>, positions: Vec<Point2>) -> crate::error::Result<Self> {
        if times.len() != positions.len() {
            return Err(crate::error::SpatialError::LengthMismatch(
                "times and positions must have equal length".into(),
            ));
        }
        Ok(Self { times, positions })
    }

    /// Number of samples.
    pub fn len(&self) -> usize {
        self.times.len()
    }

    /// Whether the trajectory contains no samples.
    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }
}

/// A snapshot at a single time step across agents.
/// Useful for per-frame analysis, visualization, or feeding into other tools.
///
/// This type is the main "view" for frame-level spatial reasoning
/// (distances, bearings, gaps, pressure, etc.).
#[derive(Clone, Debug)]
pub struct SpatialFrame {
    /// Timestamp of this frame.
    pub time: f64,
    /// Positions of all agents at this time.
    pub agent_positions: Vec<Point2>,
    /// Optional position of the focal object (e.g. ball/puck) at this time.
    pub focal: Option<Point2>,
}

/// Richer per-frame context that combines raw data with derived kinematics and
/// (future) space geometry features. Preferred name per design discussion.
#[derive(Clone, Debug)]
pub struct SpatialContext {
    pub time: f64,
    pub positions: Vec<Point2>,
    pub focal: Option<Point2>,
    pub speeds: Vec<f64>,
    pub on_ball_idx: Option<usize>,
    // Rich geometry features will be populated here as we build primitives
    pub free_space_ahead: Vec<Option<f64>>, // per agent
}

impl SpatialFrame {
    /// Number of agents in this frame.
    pub fn num_agents(&self) -> usize {
        self.agent_positions.len()
    }

    /// Position of a specific agent (by index).
    pub fn agent_pos(&self, idx: usize) -> Option<Point2> {
        self.agent_positions.get(idx).copied()
    }

    /// Position of the focal object, if present.
    pub fn focal_pos(&self) -> Option<Point2> {
        self.focal
    }

    /// Euclidean distance from agent `from` to agent `to`.
    pub fn distance_between(&self, from: usize, to: usize) -> Option<f64> {
        let a = self.agent_pos(from)?;
        let b = self.agent_pos(to)?;
        Some(a.distance(b))
    }

    /// Bearing (radians) from agent `from` toward agent `to`.
    pub fn bearing_between(&self, from: usize, to: usize) -> Option<f64> {
        let a = self.agent_pos(from)?;
        let b = self.agent_pos(to)?;
        Some(crate::geometry::bearing_between(a, b))
    }

    /// Bearing from agent `from` toward the focal object (if present).
    pub fn bearing_to_focal(&self, from: usize) -> Option<f64> {
        let a = self.agent_pos(from)?;
        let f = self.focal?;
        Some(crate::geometry::bearing_between(a, f))
    }

    /// Find the nearest other agent to `idx` and the distance.
    pub fn nearest_agent(&self, idx: usize) -> Option<(usize, f64)> {
        let my_pos = self.agent_pos(idx)?;
        let mut best = None;
        for (i, &pos) in self.agent_positions.iter().enumerate() {
            if i == idx {
                continue;
            }
            let d = my_pos.distance(pos);
            if best.map_or(true, |(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best
    }

    /// Compute all pairwise distances at this frame as a 2D ndarray (n x n, 0 on diagonal).
    /// Uses ndarray for efficient matrix operations.
    pub fn to_distance_matrix(&self) -> ndarray::Array2<f64> {
        use ndarray::Array2;
        let n = self.num_agents();
        let mut mat = Array2::<f64>::zeros((n, n));
        for i in 0..n {
            for j in (i + 1)..n {
                let d = self.agent_positions[i].distance(self.agent_positions[j]);
                mat[[i, j]] = d;
                mat[[j, i]] = d;
            }
        }
        mat
    }

    /// Compute all pairwise distances at this frame (n x n, 0 on diagonal).
    /// Vec version for convenience.
    pub fn pairwise_distances(&self) -> Vec<Vec<f64>> {
        let n = self.num_agents();
        let mut mat = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let d = self.agent_positions[i].distance(self.agent_positions[j]);
                mat[i][j] = d;
                mat[j][i] = d;
            }
        }
        mat
    }

    /// Simple free-space proxy: average distance to all other agents (larger = more "space").
    pub fn avg_free_space(&self, agent_idx: usize) -> Option<f64> {
        if agent_idx >= self.num_agents() {
            return None;
        }
        let my = self.agent_positions[agent_idx];
        let sum: f64 = self
            .agent_positions
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != agent_idx)
            .map(|(_, &p)| my.distance(p))
            .sum();
        let n = (self.num_agents() - 1) as f64;
        if n > 0.0 { Some(sum / n) } else { None }
    }

    /// Directional free space proxy: average distance to agents roughly in `direction`.
    /// angle_tolerance in radians (e.g. 1.0 ~ 57 degrees).
    pub fn free_space_in_direction(
        &self,
        agent_idx: usize,
        direction: Vec2,
        angle_tolerance: f64,
    ) -> Option<f64> {
        if agent_idx >= self.num_agents() || direction.norm() < 1e-9 {
            return None;
        }
        let my = self.agent_positions[agent_idx];
        let dir = direction.normalize();

        let relevant: Vec<f64> = self
            .agent_positions
            .iter()
            .enumerate()
            .filter_map(|(i, &p)| {
                if i == agent_idx {
                    return None;
                }
                let to_other = (p - my).normalize();
                let angle = dir.angle_to(to_other);
                if angle <= angle_tolerance {
                    Some(my.distance(p))
                } else {
                    None
                }
            })
            .collect();

        if relevant.is_empty() {
            None
        } else {
            Some(relevant.iter().sum::<f64>() / relevant.len() as f64)
        }
    }

    /// Explicit ball carrier inference for a frame.
    /// Uses a possession score: 1/(dist + eps) + positive velocity_toward_focal.
    /// Returns None if no agent is within a reasonable threshold (~2m by default).
    ///
    /// This is easier to debug/test standalone than embedding only inside the classifier.
    /// Complications:
    /// - Requires vel_toward data (or pass None for pure distance).
    /// - Threshold is somewhat arbitrary; may need per-sport or per-metadata tuning.
    /// - Without last-touch events, it's always a heuristic (velocity is proxy).
    /// - If called outside full batch context, you lose post-hoc consistency.
    pub fn infer_ball_carrier(
        &self,
        focal: Point2,
        vel_toward_focal: Option<&[f64]>,
    ) -> Option<usize> {
        let mut best = None;
        let mut best_score = f64::NEG_INFINITY;

        for (i, &p) in self.agent_positions.iter().enumerate() {
            let d = p.distance(focal);
            let vel = vel_toward_focal
                .and_then(|v| v.get(i))
                .copied()
                .unwrap_or(0.0);
            let vel_score = vel.max(0.0);
            let score = 1.0 / (d + 0.1) + 0.5 * vel_score;

            if score > best_score {
                best_score = score;
                best = Some(i);
            }
        }

        // Default ~2m threshold as discussed
        if let Some(idx) = best {
            if self.agent_positions[idx].distance(focal) > 2.0 {
                return None;
            }
        }
        best
    }

    /// Compute free space ahead using directional method (wires geometry into context).
    pub fn compute_free_space_ahead(&self, agent_idx: usize, direction: Vec2) -> Option<f64> {
        self.free_space_in_direction(agent_idx, direction, 1.0) // ~57 deg tolerance
    }
}

impl SpatialContext {
    /// Build SpatialContext from SpatialFrame + speeds (integrates kinematics + geometry).
    pub fn from_frame_and_speeds(
        frame: &SpatialFrame,
        speeds: Vec<f64>,
        focal: Option<Point2>,
    ) -> Self {
        let on_ball_idx = if let Some(f) = focal {
            frame.infer_ball_carrier(f, None)
        } else {
            None
        };
        let free_space_ahead = (0..frame.num_agents())
            .map(|i| frame.compute_free_space_ahead(i, Vec2::new(1.0, 0.0))) // default dir, can be overridden with attacking dir
            .collect();
        Self {
            time: frame.time,
            positions: frame.agent_positions.clone(),
            focal,
            speeds,
            on_ball_idx,
            free_space_ahead,
        }
    }
}

impl SpatialContext {
    /// Build a basic SpatialContext from a frame + speeds (on_ball computed simply).
    pub fn from_frame(frame: &SpatialFrame, speeds: Vec<f64>, focal: Option<Point2>) -> Self {
        let on_ball_idx = if let Some(f) = focal {
            frame
                .agent_positions
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.distance(f)
                        .partial_cmp(&b.distance(f))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
        } else {
            None
        };

        Self {
            time: frame.time,
            positions: frame.agent_positions.clone(),
            focal,
            speeds,
            on_ball_idx,
            free_space_ahead: vec![None; frame.num_agents()],
        }
    }
}

/// Container for multiple agents sharing a common time base.
/// This is the primary ergonomic batch representation for spatial analysis.
///
/// # Ergonomics
/// - Construction from `Trajectory` list or raw data
/// - Frame iteration and random access
/// - Time-based lookup
/// - Slicing
/// - Built-in classification
/// - Optional per-agent grouping (for "teams", roles, defender/attacker distinctions)
#[derive(Clone, Debug, PartialEq)]
pub struct AgentTrajectories {
    /// Shared time base (seconds).
    pub times: Vec<f64>,
    /// One series of positions per agent. All must have the same length as `times`.
    pub positions: Vec<Vec<Point2>>,
    /// Optional group/role id per agent (e.g. 0 = team A, 1 = team B).
    /// Used by classifiers and metrics to distinguish "self" vs "opponents".
    pub groups: Option<Vec<u32>>,
    /// Optional attacking direction (unit Vec2 toward opponent's goal / attacking end) per agent.
    /// Comes from metadata. Used for possession-aware semantics:
    /// same-group forward movement aligned with attacking dir = exploit/penetration (even if "behind" ball carrier).
    pub attacking_directions: Option<Vec<Vec2>>,
}

impl AgentTrajectories {
    /// Create from raw data. All agent position vectors must have the same length as `times`.
    pub fn new(times: Vec<f64>, positions: Vec<Vec<Point2>>) -> crate::error::Result<Self> {
        if positions.iter().any(|p| p.len() != times.len()) {
            return Err(crate::error::SpatialError::LengthMismatch(
                "every agent's positions must have the same length as times".into(),
            ));
        }
        Ok(Self {
            times,
            positions,
            groups: None,
            attacking_directions: None,
        })
    }

    /// Create with per-agent group/role identifiers.
    pub fn with_groups(mut self, groups: Vec<u32>) -> crate::error::Result<Self> {
        if groups.len() != self.positions.len() {
            return Err(crate::error::SpatialError::LengthMismatch(
                "groups length must match number of agents".into(),
            ));
        }
        self.groups = Some(groups);
        Ok(self)
    }

    /// Attach attacking directions (one per agent, should be unit vectors).
    pub fn with_attacking_directions(mut self, dirs: Vec<Vec2>) -> crate::error::Result<Self> {
        if dirs.len() != self.positions.len() {
            return Err(crate::error::SpatialError::LengthMismatch(
                "attacking_directions length must match number of agents".into(),
            ));
        }
        self.attacking_directions = Some(dirs);
        Ok(self)
    }

    /// Build from a list of individual `Trajectory` objects (they must share the same times).
    pub fn from_trajectories(trajs: Vec<Trajectory>) -> crate::error::Result<Self> {
        if trajs.is_empty() {
            return Ok(Self {
                times: vec![],
                positions: vec![],
                groups: None,
                attacking_directions: None,
            });
        }

        let times = trajs[0].times.clone();
        let positions = trajs
            .into_iter()
            .map(|t| {
                if t.times != times {
                    return Err(crate::error::SpatialError::LengthMismatch(
                        "all trajectories must share identical time vectors for batching".into(),
                    ));
                }
                Ok(t.positions)
            })
            .collect::<crate::error::Result<Vec<_>>>()?;

        Self::new(times, positions)
    }

    /// Number of agents.
    pub fn num_agents(&self) -> usize {
        self.positions.len()
    }

    /// Number of time samples.
    pub fn num_times(&self) -> usize {
        self.times.len()
    }

    /// Returns a frame for a specific time index (by position).
    pub fn frame(&self, t: usize) -> Option<SpatialFrame> {
        if t >= self.times.len() {
            return None;
        }
        let positions_at_t: Vec<Point2> = self.positions.iter().map(|p| p[t]).collect();
        Some(SpatialFrame {
            time: self.times[t],
            agent_positions: positions_at_t,
            focal: None,
        })
    }

    /// Get the frame whose time is closest to `query_time`.
    pub fn frame_closest(&self, query_time: f64) -> Option<SpatialFrame> {
        if self.times.is_empty() {
            return None;
        }
        // Simple linear search (fine for typical sports tracking rates)
        let mut best_idx = 0;
        let mut best_diff = (self.times[0] - query_time).abs();
        for (i, &t) in self.times.iter().enumerate().skip(1) {
            let diff = (t - query_time).abs();
            if diff < best_diff {
                best_diff = diff;
                best_idx = i;
            }
        }
        self.frame(best_idx)
    }

    /// Get positions for a specific agent.
    pub fn agent(&self, idx: usize) -> Option<&[Point2]> {
        self.positions.get(idx).map(|v| v.as_slice())
    }

    /// Convenience: infer ball carrier at a specific time index using the frame's method.
    /// Uses velocity toward focal if available via speeds or by computing delta.
    /// Returns the agent idx with highest possession score, or None if no clear carrier (e.g. >2m).
    pub fn infer_ball_carrier_at(&self, t: usize, focal: Option<Point2>) -> Option<usize> {
        if t >= self.times.len() {
            return None;
        }
        let frame = self.frame(t)?;
        let focal_pos = focal.or(frame.focal)?;
        // Compute simple vel_toward using delta if possible
        let mut vel_toward = vec![0.0; self.num_agents()];
        if t > 0 {
            for i in 0..self.num_agents() {
                if t < self.positions[i].len() {
                    let dpos = self.positions[i][t] - self.positions[i][t - 1];
                    let to_focal = focal_pos - self.positions[i][t - 1];
                    if to_focal.norm() > 1e-6 && dpos.norm() > 1e-6 {
                        let dir = dpos.normalize();
                        let unit_to = to_focal.normalize();
                        vel_toward[i] =
                            dir.dot(unit_to) * dpos.norm() / (self.times[t] - self.times[t - 1]);
                    }
                }
            }
        }
        frame.infer_ball_carrier(focal_pos, Some(&vel_toward))
    }

    /// Iterate over frames (yields owned `SpatialFrame` for each time step).
    pub fn iter_frames(&self) -> impl Iterator<Item = SpatialFrame> + '_ {
        (0..self.times.len()).filter_map(move |t| self.frame(t))
    }

    /// Iterate over all agents' position series.
    pub fn iter_agents(&self) -> impl Iterator<Item = &[Point2]> {
        self.positions.iter().map(|v| v.as_slice())
    }

    /// Slice the time dimension (returns a new owned batch, preserving groups if present).
    pub fn slice(&self, range: std::ops::Range<usize>) -> crate::error::Result<Self> {
        if range.end > self.times.len() {
            return Err(crate::error::SpatialError::InvalidParameter(
                "slice out of range".into(),
            ));
        }
        let new_times = self.times[range.clone()].to_vec();
        let new_positions: Vec<Vec<Point2>> = self
            .positions
            .iter()
            .map(|p| p[range.clone()].to_vec())
            .collect();

        let new_groups = self.groups.clone();

        let mut batch = Self::new(new_times, new_positions)?;
        batch.groups = new_groups;
        Ok(batch)
    }

    /// Classify using defaults (12m radius, 1.0s look-ahead).
    pub fn classify(&self, window_sec: f64) -> Vec<Vec<crate::decision::AgentDecision>> {
        self.classify_with_params(window_sec, 12.0, 1.0)
    }

    /// Classify with explicit parameters (supports dynamic look-ahead for post-hoc testing).
    pub fn classify_with_params(
        &self,
        window_sec: f64,
        proximity_radius: f64,
        look_ahead_sec: f64,
    ) -> Vec<Vec<crate::decision::AgentDecision>> {
        crate::decision::classify_space_actions(
            &self.positions,
            &self.times,
            None,
            window_sec,
            proximity_radius,
            look_ahead_sec,
            self.groups.as_deref(),
            self.attacking_directions.as_deref(),
        )
    }

    /// Classify with focal + parameters.
    pub fn classify_with_focal_and_params(
        &self,
        focal: &[Point2],
        window_sec: f64,
        proximity_radius: f64,
        look_ahead_sec: f64,
    ) -> Vec<Vec<crate::decision::AgentDecision>> {
        crate::decision::classify_space_actions(
            &self.positions,
            &self.times,
            Some(focal),
            window_sec,
            proximity_radius,
            look_ahead_sec,
            self.groups.as_deref(),
            self.attacking_directions.as_deref(),
        )
    }

    /// Async version of classification (default params).
    #[cfg(feature = "async")]
    pub async fn classify_async(
        &self,
        window_sec: f64,
    ) -> Vec<Vec<crate::decision::AgentDecision>> {
        self.classify_with_params_async(window_sec, 12.0, 1.0)
    }

    #[cfg(feature = "async")]
    pub async fn classify_with_params_async(
        &self,
        window_sec: f64,
        proximity_radius: f64,
        look_ahead_sec: f64,
    ) -> Vec<Vec<crate::decision::AgentDecision>> {
        let positions = self.positions.clone();
        let times = self.times.clone();
        let groups = self.groups.clone();
        tokio::task::spawn_blocking(move || {
            crate::decision::classify_space_actions(
                &positions,
                &times,
                None,
                window_sec,
                proximity_radius,
                look_ahead_sec,
                groups.as_deref(),
                self.attacking_directions.as_deref(),
            )
        })
        .await
        .unwrap_or_else(|_| vec![vec![]; self.num_agents()])
    }

    /// Derive speed (m/s) series for every agent.
    /// Each inner vec has length = num_times().saturating_sub(1)
    pub fn speeds(&self) -> Vec<Vec<f64>> {
        self.positions
            .iter()
            .map(|p| crate::kinematics::derive_speeds(p, &self.times))
            .collect()
    }

    /// Per-agent (accels, decels) using the given threshold (m/s²).
    pub fn accel_decel_counts(&self, accel_threshold: f64) -> Vec<(usize, usize)> {
        self.positions
            .iter()
            .map(|p| {
                let sp = crate::kinematics::derive_speeds(p, &self.times);
                crate::kinematics::count_accelerations_decelerations(
                    &sp,
                    &self.times,
                    accel_threshold,
                )
            })
            .collect()
    }

    /// Per-agent (peak_pace, Vec<relative_pace 0..1> )
    pub fn normalized_peak_paces(&self) -> Vec<(f64, Vec<f64>)> {
        self.speeds()
            .into_iter()
            .map(|sp| crate::kinematics::normalize_to_peak_pace(&sp))
            .collect()
    }
}

/// Summary for a single player/agent.
/// Combines spatial-derived signals with generic load metrics from loadsym.
#[derive(Debug, Clone)]
pub struct PlayerSummary {
    pub player_idx: usize,
    pub group: Option<u32>,
    pub total_distance: f64,
    pub avg_speed: f64,
    pub max_speed: f64,
    pub accel_count: usize,
    pub decel_count: usize,
    pub estimated_load: f64,
    /// Average distance to focal object across the session (if focal provided).
    pub avg_dist_to_focal: Option<f64>,
    // Future: time_as_on_ball_carrier, high_speed_distance, etc.
}

/// Team/group level aggregate.
#[derive(Debug, Clone)]
pub struct GroupSummary {
    pub group: u32,
    pub num_players: usize,
    pub total_distance: f64,
    pub avg_max_speed: f64,
    pub total_accels: usize,
    pub total_decels: usize,
    pub total_estimated_load: f64,
    /// Average pairwise distance between members of this group (cohesion / spread metric).
    pub avg_intra_group_distance: f64,
}

impl AgentTrajectories {
    /// Compute per-player summaries.
    ///
    /// Uses loadsym for the generic movement load part.
    /// If focal is provided, also computes average distance to focal per player.
    pub fn per_player_summaries(
        &self,
        accel_threshold: f64,
        action_weight: f64,
        focal: Option<&[Point2]>,
    ) -> Vec<PlayerSummary> {
        let speeds_all = self.speeds();
        let counts = self.accel_decel_counts(accel_threshold);

        (0..self.num_agents())
            .map(|i| {
                let speeds = &speeds_all[i];
                let (ac, dc) = counts[i];
                let metrics = symworx_loadsym::compute_movement_load_metrics(
                    speeds,
                    &self.times,
                    ac,
                    dc,
                    action_weight,
                );

                let group = self.groups.as_ref().and_then(|g| g.get(i).copied());

                let avg_dist_to_focal = focal.map(|f| {
                    let mut sum = 0.0;
                    let mut n = 0;
                    for (t, &pos) in self.positions[i].iter().enumerate() {
                        if t < f.len() {
                            sum += pos.distance(f[t]);
                            n += 1;
                        }
                    }
                    if n > 0 { sum / n as f64 } else { 0.0 }
                });

                PlayerSummary {
                    player_idx: i,
                    group,
                    total_distance: metrics.total_distance,
                    avg_speed: metrics.avg_speed,
                    max_speed: metrics.max_speed,
                    accel_count: metrics.accel_count,
                    decel_count: metrics.decel_count,
                    estimated_load: metrics.estimated_load,
                    avg_dist_to_focal,
                }
            })
            .collect()
    }

    /// Aggregate summaries by group.
    pub fn per_group_summaries(
        &self,
        accel_threshold: f64,
        action_weight: f64,
        focal: Option<&[Point2]>,
    ) -> Vec<GroupSummary> {
        use std::collections::BTreeMap;

        let player_sums = self.per_player_summaries(accel_threshold, action_weight, focal);

        let mut by_group: BTreeMap<u32, Vec<&PlayerSummary>> = BTreeMap::new();

        for ps in &player_sums {
            if let Some(g) = ps.group {
                by_group.entry(g).or_default().push(ps);
            }
        }

        by_group
            .into_iter()
            .map(|(group, players)| {
                let num = players.len();
                let total_dist: f64 = players.iter().map(|p| p.total_distance).sum();
                let avg_max: f64 = if num > 0 {
                    players.iter().map(|p| p.max_speed).sum::<f64>() / num as f64
                } else {
                    0.0
                };
                let total_ac: usize = players.iter().map(|p| p.accel_count).sum();
                let total_dc: usize = players.iter().map(|p| p.decel_count).sum();
                let total_load: f64 = players.iter().map(|p| p.estimated_load).sum();

                // Compute average intra-group pairwise distance across all time (simple spatial cohesion)
                let mut intra_sum = 0.0_f64;
                let mut intra_count = 0_usize;
                let group_indices: Vec<usize> = player_sums
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, ps)| {
                        if ps.group == Some(group) {
                            Some(idx)
                        } else {
                            None
                        }
                    })
                    .collect();

                for t in 0..self.num_times() {
                    for ii in 0..group_indices.len() {
                        for jj in (ii + 1)..group_indices.len() {
                            let i = group_indices[ii];
                            let j = group_indices[jj];
                            if t < self.positions[i].len() && t < self.positions[j].len() {
                                let d = self.positions[i][t].distance(self.positions[j][t]);
                                intra_sum += d;
                                intra_count += 1;
                            }
                        }
                    }
                }
                let avg_intra = if intra_count > 0 {
                    intra_sum / intra_count as f64
                } else {
                    0.0
                };

                GroupSummary {
                    group,
                    num_players: num,
                    total_distance: total_dist,
                    avg_max_speed: avg_max,
                    total_accels: total_ac,
                    total_decels: total_dc,
                    total_estimated_load: total_load,
                    avg_intra_group_distance: avg_intra,
                }
            })
            .collect()
    }
}
