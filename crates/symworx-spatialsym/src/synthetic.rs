// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Synthetic data generation for spatial analysis (options 1-3).
//!
//! 1. Parametric linear + simple curves
//! 2. Using symworx-math primitives (random, series, oscillators where useful)
//! 3. Event-driven scenario builder
//!
//! These produce data usable with AgentTrajectories, including groups and
//! attacking directions for possession-aware classification.

use crate::{
    geometry::{
        Point2,
        Vec2,
    },
    trajectory::AgentTrajectories,
};

/// Generate a parametric linear trajectory (option 1 base).
/// Starts at `start`, moves with constant `velocity` for `duration` at `dt`.
pub fn generate_linear_trajectory(
    start: Point2,
    velocity: Vec2,
    duration: f64,
    dt: f64,
) -> Vec<Point2> {
    let n = (duration / dt).ceil() as usize;
    (0..n)
        .map(|i| {
            let t = i as f64 * dt;
            start + velocity * t
        })
        .collect()
}

/// Generate a simple curved trajectory using sine (parametric curve, option 1/2).
/// Amplitude and frequency control the curve.
pub fn generate_curved_trajectory(
    start: Point2,
    base_velocity: Vec2,
    duration: f64,
    dt: f64,
    amplitude: f64,
    frequency: f64,
) -> Vec<Point2> {
    let n = (duration / dt).ceil() as usize;
    (0..n)
        .map(|i| {
            let t = i as f64 * dt;
            let x = start.x + base_velocity.x * t;
            let y = start.y + base_velocity.y * t + amplitude * (frequency * t).sin();
            Point2::new(x, y)
        })
        .collect()
}

/// Noisy trajectory using simple deterministic pattern + math (option 2, no extra rand dep).
pub fn generate_noisy_trajectory(
    start: Point2,
    base_velocity: Vec2,
    duration: f64,
    dt: f64,
    noise_amp: f64,
) -> Vec<Point2> {
    let n = (duration / dt).ceil() as usize;
    (0..n)
        .map(|i| {
            let t = i as f64 * dt;
            let base = start + base_velocity * t;
            // deterministic "noise" using sin for reproducibility
            let noise_x = (t * 7.3).sin() * noise_amp;
            let noise_y = (t * 11.7).cos() * noise_amp * 0.6;
            Point2::new(base.x + noise_x, base.y + noise_y)
        })
        .collect()
}

/// Event for event-driven generator (option 3).
#[derive(Clone, Debug)]
pub enum SpatialEvent {
    /// Agent starts moving toward a target position at given speed.
    StartRun {
        agent: usize,
        target: Point2,
        speed: f64,
        start_time: f64,
    },
    /// Ball is passed from one agent to another at time.
    Pass { from: usize, to: usize, time: f64 },
    /// Defender starts closing on target.
    Close {
        agent: usize,
        target: usize,
        speed: f64,
        start_time: f64,
    },
}

/// Simple event-driven generator (option 3).
/// Starts with initial positions, applies events to simulate paths.
/// This is a skeleton; extend with more physics.
pub fn generate_event_driven(
    initial_positions: Vec<Point2>,
    focal_start: Point2,
    events: Vec<SpatialEvent>,
    duration: f64,
    dt: f64,
) -> (Vec<f64>, Vec<Vec<Point2>>, Vec<Point2>) {
    let n_agents = initial_positions.len();
    let n_steps = (duration / dt).ceil() as usize;
    let times: Vec<f64> = (0..n_steps).map(|i| i as f64 * dt).collect();

    let mut positions: Vec<Vec<Point2>> = initial_positions.into_iter().map(|p| vec![p]).collect();
    let mut focal = vec![focal_start];

    // Very basic simulation: move agents toward targets or with ball.
    // In real impl, maintain current velocity per agent, update on events.
    let mut current_targets: Vec<Option<Point2>> = vec![None; n_agents];
    let mut current_speeds: Vec<f64> = vec![0.0; n_agents];
    let mut ball_carrier: Option<usize> = Some(0); // assume start with 0

    for step in 1..n_steps {
        let t = times[step];
        let _prev_t = times[step - 1];

        // Process events at this time
        for ev in &events {
            match ev {
                SpatialEvent::StartRun {
                    agent,
                    target,
                    speed,
                    start_time,
                } if (start_time - t).abs() < dt / 2.0 => {
                    current_targets[*agent] = Some(*target);
                    current_speeds[*agent] = *speed;
                }
                SpatialEvent::Pass { from, to, time } if (time - t).abs() < dt / 2.0 => {
                    ball_carrier = Some(*to);
                    current_targets[*from] = None;
                    current_speeds[*from] = 0.0;
                }
                SpatialEvent::Close {
                    agent,
                    target,
                    speed,
                    start_time,
                } if (start_time - t).abs() < dt / 2.0 => {
                    // Defender moves toward current pos of target
                    current_targets[*agent] = Some(positions[*target][step - 1]);
                    current_speeds[*agent] = *speed;
                }
                _ => {}
            }
        }

        // Update positions
        for agent in 0..n_agents {
            let prev_pos = positions[agent][step - 1];
            let mut new_pos = prev_pos;

            if let Some(target) = current_targets[agent] {
                let to_target = target - prev_pos;
                if to_target.norm() > 0.1 {
                    let dir = to_target.normalize();
                    let move_dist = current_speeds[agent] * dt;
                    new_pos = prev_pos + dir * move_dist.min(to_target.norm());
                }
            } else if ball_carrier == Some(agent) {
                // Carrier keeps moving a bit if no target
                // For demo, assume continues previous direction or stays.
            }

            positions[agent].push(new_pos);
        }

        // Update focal (simple: follows carrier or moves to receiver on pass)
        let carrier = ball_carrier.unwrap_or(0);
        let carrier_pos = *positions[carrier].last().unwrap();
        focal.push(carrier_pos + Vec2::new(0.3, 0.1) * ((t * 2.0).sin() * 0.5)); // slight wobble for demo
    }

    (times, positions, focal)
}

/// Build a full AgentTrajectories + metadata from generated data.
/// Convenience for analysis.
pub fn build_agent_trajectories(
    times: Vec<f64>,
    positions: Vec<Vec<Point2>>,
    groups: Vec<u32>,
    attacking_directions: Vec<Vec2>,
    focal: Vec<Point2>,
    playing_dimensions: Option<crate::space::PlayingDimensions>,
    goal_positions: Option<Vec<Point2>>,
) -> (AgentTrajectories, Vec<Point2>) {
    let mut batch = AgentTrajectories::new(times, positions)
        .expect("positions must match times length")
        .with_groups(groups)
        .expect("groups length must match")
        .with_attacking_directions(attacking_directions)
        .expect("directions length must match");

    if let Some(dims) = playing_dimensions {
        batch = batch.with_playing_dimensions(dims);
    }
    if let Some(g) = goal_positions {
        batch = batch
            .with_goal_positions(g)
            .expect("goal_positions length must match");
    }

    (batch, focal)
}

/// Ground truth labels for testing the classifier against known scenarios.
/// Returns per-agent expected SpaceAction over time.
pub fn generate_ground_truth(
    n_agents: usize,
    n_steps: usize,
    scenario_type: &str,
) -> Vec<Vec<crate::decision::SpaceAction>> {
    let mut labels = vec![vec![crate::decision::SpaceAction::Neutral; n_steps]; n_agents];
    match scenario_type {
        "dribble_pen" => {
            for t in 3..n_steps.min(8) {
                labels[0][t] = crate::decision::SpaceAction::Penetration;
            }
        }
        "support_run" => {
            for t in 5..n_steps.min(10) {
                labels[1][t] = crate::decision::SpaceAction::Penetration; // off ball exploit
            }
        }
        "close_pressure" => {
            for t in 7..n_steps {
                labels[2][t] = crate::decision::SpaceAction::Pressure;
            }
        }
        "pass_then_press" => {
            for t in 4..7 {
                labels[0][t] = crate::decision::SpaceAction::Expansion; // create/pass
            }
            for t in 8..n_steps.min(12) {
                labels[1][t] = crate::decision::SpaceAction::Pressure; // receiver pressed
            }
        }
        "create_chance" => {
            // Attacker creates a scoring opportunity (e.g. gets open near target)
            for t in 6..n_steps.min(11) {
                labels[0][t] = crate::decision::SpaceAction::Creation;
            }
        }
        "conversion" => {
            // Successful score / conversion
            for t in 9..n_steps.min(12) {
                labels[0][t] = crate::decision::SpaceAction::Conversion;
            }
        }
        "deny_chance" => {
            // Defender prevents a scoring opportunity
            for t in 5..n_steps.min(10) {
                labels[2][t] = crate::decision::SpaceAction::Prevention;
            }
        }
        "scoring_sequence" => {
            // Longer sequence demonstrating Creation (run creates chance) + Denial (defender closes) + Conversion (finishes)
            // Agent 0 creates by running into space near goal area
            for t in 8..15 {
                if t < n_steps {
                    labels[0][t] = crate::decision::SpaceAction::Creation;
                }
            }
            // Agent 1 receives and converts
            for t in 16..20 {
                if t < n_steps {
                    labels[1][t] = crate::decision::SpaceAction::Conversion;
                }
            }
            // Defender (agent 2) tries to deny the space
            for t in 10..18 {
                if t < n_steps {
                    labels[2][t] = crate::decision::SpaceAction::Prevention;
                }
            }
        }
        _ => {}
    }
    labels
}
