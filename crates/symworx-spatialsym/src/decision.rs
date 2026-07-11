// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Post-hoc classification of spatial decision-making actions.
//!
//! These labels are derived using historical (past) and future information about movement.
//! The public API remains completely sport-agnostic.
//!
//! ## Naming & Language Conventions
//!
//! - **Public types** (SpaceAction, classify_*, etc.) use neutral terms:
//!   - `Expansion`   (create space laterally or backwards)
//!   - `Penetration` (exploit space forward / through gaps)
//!   - `Denial`      (deny space / close lanes or channels)
//!   - `Pressure`    (close space / immediate approach to contest or regain)
//!   - `Creation`    (create a scoring opportunity)
//!   - `Conversion`  (successful score / convert opportunity)
//!   - `Prevention`  (deny an opponent's scoring opportunity)
//! - In **comments, docs, examples and internal logic** we may use professional
//!   sports language (especially soccer/football) for clarity:
//!   - "on-ball player" / "ball carrier"
//!   - "supporting runner" / "attacking space to receive"
//!   - "drawing defenders out of shape"
//!   - "carrying / dribbling"
//!   - "immediate press after reception"
//!   - "playing the ball into space"
//!
//! ## Core Principle
//!
//! At every moment we look at the **on-ball player + the players closest to them**
//! (typically within a 10-15m "action radius"). We also use post-hoc information
//! to correct earlier labels (e.g. a pass that looked like exploitation but was
//! immediately closed down after reception).

/// Questions and circumstances we must account for when labeling actions
/// (using soccer terminology for concreteness; the implementation stays generic).
///
/// ## On-ball player (ball carrier)
/// - Is the player carrying/dribbling forward into open space? (Penetration)
/// - Is the player carrying sideways or backwards to create width or reset? (Expansion)
/// - Is the player playing a forward pass into space for a teammate to run onto?
///   (The passer's action may be "creating" or "exploiting via pass")
///
/// ## Off-ball players (especially within 10-15m of the ball)
/// - A player making a run into a gap to receive a pass (Penetration / attacking space)
/// - A player checking back or moving wide to create a passing option or stretch the defense (Expansion)
/// - A player moving to close down a passing lane or the ball carrier (Denial)
/// - A defender stepping up to press the ball carrier immediately (Pressure)
///
/// ## Post-hoc reclassification rules (important)
/// - Player A (on ball) plays a forward pass to Player B who was >15m away.
/// - Immediately after reception, the defender who was marking A sprints and presses B.
/// - In this case we should consider re-evaluating Player A's previous action at the moment of the pass.
///   Was it truly "exploitation", or was it playing into a trap / poor decision?
///
/// ## Broader circumstances to handle over time
/// - Player on the ball waits / holds possession (may be Neutral or creating time)
/// - Player receives the ball under immediate pressure (Pressure on the receiver)
/// - Player makes a decoy run that draws two defenders away (Expansion / drawing shape)
/// - Player on the ball plays a safe sideways pass while under pressure
/// - Counter-attack situations with large spaces behind the defensive line
/// - Build-up play where multiple short passes are used to create space higher up
pub mod circumstances {
    // This module exists purely for documentation and future test scenarios.
}

/// Sport-agnostic classification of an agent's observed space-management action
/// at a given time (computed post-hoc using past + future trajectory context).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpaceAction {
    /// Lateral or backward movement that opens usable space ("create space").
    Expansion,
    /// Forward movement through gaps or into lower-density regions ("exploit space").
    Penetration,
    /// Movement/positioning that restricts lanes or options for others ("deny space").
    Denial,
    /// Direct reduction of distance to an opponent or focal object ("close space").
    Pressure,
    /// No clear dominant space action detected (or below threshold).
    Neutral,

    // Higher-level outcome categories (sport-agnostic)
    /// Action that creates a clear, high-value scoring opportunity against the target.
    Creation,
    /// Successful conversion of a scoring opportunity (sport-agnostic for "goal", "basket", "point", etc.).
    Conversion,
    /// Action that prevents an opponent from creating or converting a scoring opportunity.
    Prevention,
}

/// Richer per-agent decision output for classifier maturity.
#[derive(Clone, Debug)]
pub struct AgentDecision {
    pub action: SpaceAction,
    /// Optional confidence [0,1] or strength score.
    pub confidence: Option<f64>,
    /// Key features that drove the decision (for explainability and richer rules).
    pub features: DecisionFeatures,
}

/// Container for features used in decision making. These can come from
/// kinematics + richer space geometry primitives.
#[derive(Clone, Debug, Default)]
pub struct DecisionFeatures {
    /// Current speed (m/s).
    pub speed: f64,
    /// Forward component of movement (cos of bearing, positive = more forward).
    pub forward_component: f64,
    /// Distance to nearest opponent (using groups if provided).
    pub nearest_opponent_dist: Option<f64>,
    /// Proxy for free/controlled space in the direction of movement.
    /// (Will be populated by richer geometry primitives.)
    pub free_space_ahead: Option<f64>,
    /// Relative pace (speed / player peak) if normalized.
    pub relative_pace: Option<f64>,
    /// Whether this agent is considered the ball carrier.
    pub is_ball_carrier: bool,
    /// Velocity component toward the focal (positive = moving toward).
    pub vel_toward_focal: Option<f64>,
}

impl SpaceAction {
    /// Human-readable description (useful for logging / UI).
    pub fn description(&self) -> &'static str {
        match self {
            SpaceAction::Expansion => "Expansion (create space)",
            SpaceAction::Penetration => "Penetration (exploit space)",
            SpaceAction::Denial => "Denial (deny space)",
            SpaceAction::Pressure => "Pressure (close space)",
            SpaceAction::Neutral => "Neutral",
            SpaceAction::Creation => "Creation (create scoring opportunity)",
            SpaceAction::Conversion => "Conversion (successful score)",
            SpaceAction::Prevention => "Prevention (deny scoring opportunity)",
        }
    }
}

/// Classify space actions for a single trajectory (legacy single-agent API).
pub fn classify_single_trajectory(
    positions: &[crate::geometry::Point2],
    times: &[f64],
    window_sec: f64,
) -> Vec<AgentDecision> {
    classify_single_trajectory_with_params(positions, times, window_sec, 12.0, 1.0, None, None)
}

/// Single-trajectory version with more parameters for maturity.
pub fn classify_single_trajectory_with_params(
    positions: &[crate::geometry::Point2],
    times: &[f64],
    window_sec: f64,
    proximity_radius: f64,
    look_ahead_sec: f64,
    groups: Option<&[u32]>,
    attacking_directions: Option<&[crate::geometry::Vec2]>,
) -> Vec<AgentDecision> {
    let trajs = vec![positions.to_vec()];
    let results = classify_space_actions(
        &trajs,
        times,
        None,
        window_sec,
        proximity_radius,
        look_ahead_sec,
        groups,
        attacking_directions,
        None,
        None,
    );
    results.into_iter().next().unwrap_or_default()
}

/// Improved post-hoc classifier that focuses on the **on-ball player + nearby players**.
///
/// We identify the player closest to the focal object ("on-ball player") and also
/// consider players within `proximity_radius` of them.
///
/// Special post-hoc rule (using future information):
/// If a player plays the ball to someone outside the radius, and that receiver
/// is then immediately pressed by the player who was previously marking the passer,
/// we re-evaluate the original action at the moment of the pass.
///
/// `proximity_radius` example values (professional soccer contexts): 10.0–15.0 meters.
pub fn classify_space_actions(
    agent_trajectories: &[Vec<crate::geometry::Point2>],
    times: &[f64],
    focal_trajectory: Option<&[crate::geometry::Point2]>,
    window_sec: f64,
    proximity_radius: f64,
    look_ahead_sec: f64,
    groups: Option<&[u32]>,
    attacking_directions: Option<&[crate::geometry::Vec2]>,
    _playing_dimensions: Option<&crate::space::PlayingDimensions>,
    goal_positions: Option<&[crate::geometry::Point2]>,
) -> Vec<Vec<AgentDecision>> {
    use crate::kinematics::{
        future_bearings,
        past_bearings,
    };

    let n_agents = agent_trajectories.len();
    if n_agents == 0 || times.is_empty() {
        return vec![vec![]; n_agents];
    }
    let n_times = times.len();

    // Helper: are these two agents opponents? (different group id)
    // If no groups info, assume all interactions are relevant (opponents).
    let is_opponent = |a: usize, b: usize| -> bool {
        match groups {
            Some(g) if a < g.len() && b < g.len() => g[a] != g[b],
            _ => true,
        }
    };

    // Precompute per-agent past/future bearings + speeds
    let mut all_past: Vec<Vec<Option<f64>>> = Vec::new();
    let mut all_future: Vec<Vec<Option<f64>>> = Vec::new();
    let mut all_speeds: Vec<Vec<f64>> = Vec::new();
    let mut all_vel_toward_focal: Vec<Vec<Option<f64>>> = Vec::new();

    for traj in agent_trajectories {
        let past = past_bearings(traj, times, window_sec);
        let future = future_bearings(traj, times, window_sec);

        let mut speeds = vec![0.0; n_times];
        let mut vel_toward = vec![None; n_times];

        for i in 1..n_times {
            let dt = times[i] - times[i - 1];
            if dt > 0.0 {
                let d = traj[i] - traj[i - 1];
                speeds[i] = d.norm() / dt;
            }
        }

        // Ball carrier: include velocity toward focal
        if let Some(focal) = focal_trajectory {
            for i in 0..n_times {
                if i < focal.len() {
                    let to_focal = focal[i] - traj[i];
                    if to_focal.norm() > 1e-6 {
                        if i > 0 && speeds[i] > 0.0 {
                            let dir = (traj[i] - traj[i - 1]).normalize();
                            let unit_to_focal = to_focal.normalize();
                            vel_toward[i] = Some(dir.dot(unit_to_focal) * speeds[i]);
                        }
                    }
                }
            }
        }

        all_past.push(past);
        all_future.push(future);
        all_speeds.push(speeds);
        all_vel_toward_focal.push(vel_toward);
    }

    let mut results = vec![
        vec![
            AgentDecision {
                action: SpaceAction::Neutral,
                confidence: None,
                features: DecisionFeatures::default(),
            };
            n_times
        ];
        n_agents
    ];

    // Precompute on-ball player and nearby players per frame
    let mut on_ball_at_t: Vec<Option<usize>> = vec![None; n_times];
    let mut nearby_at_t: Vec<Vec<usize>> = vec![vec![]; n_times];

    if let Some(focal) = focal_trajectory {
        for t in 0..n_times.min(focal.len()) {
            // Possession score: combines proximity + velocity toward focal.
            // score = 1 / (dist + eps) + positive vel_toward component.
            // This is a soft "ball carrier" likelihood.
            let mut best_agent = None;
            let mut best_score = f64::NEG_INFINITY;
            let mut best_dist = f64::INFINITY;

            for a in 0..n_agents {
                if t >= agent_trajectories[a].len() {
                    continue;
                }
                let d = (agent_trajectories[a][t] - focal[t]).norm();
                let vel = all_vel_toward_focal[a][t].unwrap_or(0.0);
                let vel_score = vel.max(0.0);
                let score = 1.0 / (d + 0.1) + 0.5 * vel_score;

                if score > best_score {
                    best_score = score;
                    best_agent = Some(a);
                    best_dist = d;
                }
            }

            // Default threshold ~2m: if the best is too far, no clear carrier.
            // User suggestion; adjustable via look_ahead or future config.
            let possession_threshold = 2.0;
            if best_dist > possession_threshold {
                best_agent = None;
            }

            on_ball_at_t[t] = best_agent;

            if let Some(ball_carrier) = best_agent {
                let carrier_pos = agent_trajectories[ball_carrier][t];
                let mut nearby = vec![];

                for a in 0..n_agents {
                    if a == ball_carrier {
                        continue;
                    }
                    if t >= agent_trajectories[a].len() {
                        continue;
                    }
                    let d = (agent_trajectories[a][t] - carrier_pos).norm();
                    if d <= proximity_radius {
                        nearby.push(a);
                    }
                }
                nearby_at_t[t] = nearby;
            }
        }
    }

    for t in 0..n_times {
        let on_ball = on_ball_at_t[t];
        let nearby = &nearby_at_t[t];

        // Determine possession state for the frame (based on ball carrier group)
        let in_possession_group =
            on_ball.and_then(|carrier| groups.and_then(|g| g.get(carrier).copied()));

        for a in 0..n_agents {
            let pos = agent_trajectories[a][t];
            let speed = all_speeds[a][t];
            let past_b = all_past[a][t];
            let fut_b = all_future[a][t];
            let vel_toward = all_vel_toward_focal[a][t];

            let is_on_ball = on_ball == Some(a);
            let is_nearby = nearby.contains(&a);

            if !is_on_ball && !is_nearby {
                results[a][t] = AgentDecision {
                    action: SpaceAction::Neutral,
                    confidence: Some(0.0),
                    features: DecisionFeatures {
                        speed,
                        is_ball_carrier: false,
                        vel_toward_focal: vel_toward,
                        ..Default::default()
                    },
                };
                continue;
            }

            let _focal_dist = focal_trajectory.and_then(|f| {
                if t < f.len() {
                    Some((f[t] - pos).norm())
                } else {
                    None
                }
            });

            // Distance to this agent's goal (if provided). Used for scoring-opportunity logic.
            let dist_to_goal = goal_positions.and_then(|g| {
                if a < g.len() {
                    Some((g[a] - pos).norm())
                } else {
                    None
                }
            });

            // Nearest opponent (groups aware)
            let mut nearest_dist = f64::INFINITY;
            for &other in nearby.iter() {
                if other == a {
                    continue;
                }
                if t >= agent_trajectories[other].len() {
                    continue;
                }
                if !is_opponent(a, other) {
                    continue;
                }
                let d = (agent_trajectories[other][t] - pos).norm();
                if d < nearest_dist {
                    nearest_dist = d;
                }
            }

            let just_received_under_pressure = if is_nearby && t > 0 {
                let was_outside = if let Some(prev_ball) = on_ball_at_t[t.saturating_sub(1)] {
                    let prev_pos = agent_trajectories[prev_ball][t - 1];
                    (agent_trajectories[a][t - 1] - prev_pos).norm() > proximity_radius * 1.05
                } else {
                    false
                };

                let now_pressed = nearest_dist < 7.0 && speed > 0.6;
                was_outside && now_pressed
            } else {
                false
            };

            // Possession-aware direction semantics
            let player_group = groups.and_then(|g| g.get(a).copied());
            let same_possession = in_possession_group == player_group;

            // Use attacking direction from metadata if available, else fall back to absolute bearing
            let forward = if let (Some(_pb), Some(fb)) = (past_b, fut_b) {
                if let Some(dirs) = attacking_directions {
                    if a < dirs.len() {
                        let att = dirs[a];
                        // cos of angle between future bearing and attacking dir
                        (fb.cos() * att.x + fb.sin() * att.y).max(-1.0).min(1.0)
                    } else {
                        fb.cos()
                    }
                } else {
                    fb.cos()
                }
            } else {
                0.0
            };

            // Simple proxy for "dangerous / scoring zone" using attacking direction.
            // When we have attacking_directions we can estimate how much this player has advanced
            // toward the goal (useful for gating Creation/Conversion/Prevention).
            // Progress toward the attack direction; currently gated on dirs existing
            // so we can later use dirs[a] as a richer goal-progress signal.
            let goal_progress = if let Some(dirs) = attacking_directions {
                if a < dirs.len() {
                    let _att = dirs[a];
                    // Use the forward component itself as progress signal (higher = deeper in attacking half)
                    forward.max(0.0)
                } else {
                    forward.max(0.0)
                }
            } else {
                forward.max(0.0)
            };

            // Wire up avg_free_space as first-class geometry feature (avg distance to others)
            let free_space = {
                let my_pos = pos;
                let sum: f64 = (0..n_agents)
                    .filter(|&i| i != a)
                    .map(|i| my_pos.distance(agent_trajectories[i][t]))
                    .sum();
                let cnt = (n_agents.saturating_sub(1)) as f64;
                if cnt > 0.0 { sum / cnt } else { 0.0 }
            };

            // Build features — now using current avg_free_space
            let features = DecisionFeatures {
                speed,
                forward_component: forward,
                nearest_opponent_dist: if nearest_dist.is_finite() {
                    Some(nearest_dist)
                } else {
                    None
                },
                free_space_ahead: Some(free_space),
                relative_pace: None,
                is_ball_carrier: is_on_ball,
                vel_toward_focal: vel_toward,
            };

            // Core rules (speed/accel impact, possession-aware)
            // Scoring opportunity categories (Creation / Conversion / Prevention) use stronger signals
            // and awareness of attacking direction + goal progress + explicit goal distance when available.
            let action = if just_received_under_pressure {
                SpaceAction::Pressure
            } else if is_on_ball {
                // Use explicit dist_to_goal when provided, else fall back to goal_progress heuristic.
                let near_goal = dist_to_goal.map_or(goal_progress > 0.55, |d| d < 12.0); // ~12m "box"
                if forward > 0.72 && speed > 2.8 && nearest_dist > 3.5 && near_goal {
                    if forward > 0.85 && speed > 4.0 && nearest_dist > 6.0 {
                        SpaceAction::Conversion
                    } else {
                        SpaceAction::Creation
                    }
                } else if forward > 0.6 && speed > 0.5 && nearest_dist > proximity_radius * 0.5 {
                    SpaceAction::Penetration
                } else if forward.abs() < 0.5 && speed > 0.25 {
                    SpaceAction::Expansion
                } else if nearest_dist < 6.0 && speed > 0.7 {
                    SpaceAction::Pressure
                } else {
                    SpaceAction::Neutral
                }
            } else if is_nearby {
                if same_possession {
                    // Off-ball attacker making a dangerous run that creates a scoring chance
                    let near_goal = dist_to_goal.map_or(goal_progress > 0.4, |d| d < 15.0);
                    let creating_danger =
                        near_goal && forward > 0.45 && speed > 2.2 && nearest_dist > 4.5;
                    if creating_danger {
                        SpaceAction::Creation
                    } else if forward > 0.3 && speed > 0.3 {
                        SpaceAction::Penetration
                    } else {
                        SpaceAction::Expansion
                    }
                } else {
                    // Defender actively preventing a goal-scoring opportunity
                    let near_goal = dist_to_goal.map_or(goal_progress > 0.3, |d| d < 15.0);
                    let denying_danger = near_goal && (forward < -0.05 || nearest_dist < 7.5);
                    if denying_danger {
                        SpaceAction::Prevention
                    } else if nearest_dist < 5.0 {
                        SpaceAction::Pressure
                    } else {
                        SpaceAction::Neutral
                    }
                }
            } else {
                SpaceAction::Neutral
            };

            // Simple confidence based on signals (speed, space, etc.)
            let confidence = Some(
                (0.3 + 0.4 * (speed / 5.0).min(1.0) + if nearest_dist > 10.0 { 0.3 } else { 0.0 })
                    .min(1.0),
            );

            results[a][t] = AgentDecision {
                action,
                confidence,
                features,
            };
        }
    }

    // Post-hoc reclassification using configurable look_ahead_sec
    // (supports re-evaluating / providing additional context to past events)
    let look_ahead_frames = if look_ahead_sec > 0.0 && !times.is_empty() {
        let avg_dt = if n_times > 1 {
            (times[n_times - 1] - times[0]) / (n_times as f64 - 1.0)
        } else {
            0.04
        };
        (look_ahead_sec / avg_dt).max(1.0) as usize
    } else {
        1
    };

    for t in 0..n_times.saturating_sub(look_ahead_frames) {
        if let (Some(carrier), Some(receiver)) =
            (on_ball_at_t[t], on_ball_at_t[t + look_ahead_frames])
        {
            if carrier != receiver {
                let receiver_now_pressed = {
                    let mut min_d = f64::INFINITY;
                    for other in 0..n_agents {
                        if other == receiver {
                            continue;
                        }
                        if t + look_ahead_frames >= agent_trajectories[other].len() {
                            continue;
                        }
                        if !is_opponent(receiver, other) {
                            continue;
                        }
                        let d = (agent_trajectories[other][t + look_ahead_frames]
                            - agent_trajectories[receiver][t + look_ahead_frames])
                            .norm();
                        if d < min_d {
                            min_d = d;
                        }
                    }
                    min_d < 7.0
                };

                if receiver_now_pressed {
                    if let Some(dec) = results.get_mut(carrier).and_then(|v| v.get_mut(t)) {
                        match dec.action {
                            SpaceAction::Penetration
                            | SpaceAction::Creation
                            | SpaceAction::Conversion => {
                                dec.action = SpaceAction::Neutral;
                                // Reclassified because the receiver was under immediate pressure
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_action_variants_and_desc() {
        assert_eq!(
            SpaceAction::Expansion.description(),
            "Expansion (create space)"
        );
        assert_eq!(
            SpaceAction::Penetration.description(),
            "Penetration (exploit space)"
        );
        assert_eq!(SpaceAction::Denial.description(), "Denial (deny space)");
        assert_eq!(
            SpaceAction::Pressure.description(),
            "Pressure (close space)"
        );
        assert_eq!(
            SpaceAction::Creation.description(),
            "Creation (create scoring opportunity)"
        );
        assert_eq!(
            SpaceAction::Conversion.description(),
            "Conversion (successful score)"
        );
        assert_eq!(
            SpaceAction::Prevention.description(),
            "Prevention (deny scoring opportunity)"
        );
    }
}
