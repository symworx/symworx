// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Formation-seeking 11v11 sequence of play (1 Hz default, 3 minutes).
//!
//! This is **not** a physics match engine. Agents seek scripted formation slots
//! that compress/expand with a phase table; the ball follows a planted carrier
//! (with 1 s linear flight on passes). Planted [`SpaceAction`] labels come from
//! the script, not from the kinematics — that split is the point of the
//! evaluation path.

use crate::{
    decision::SpaceAction,
    geometry::{
        Point2,
        Vec2,
    },
    space::PlayingDimensions,
    synthetic::build_agent_trajectories,
    trajectory::AgentTrajectories,
};

/// Default sequence length (seconds).
pub const PLAY_11V11_DURATION_SEC: f64 = 180.0;
/// Default sample interval (seconds) — tracking-grade, not optical 25 Hz.
pub const PLAY_11V11_DT_SEC: f64 = 1.0;

const N_AGENTS: usize = 22;
const G0: u32 = 0;
const G1: u32 = 1;

/// Generated sequence plus planted labels and event tags.
#[derive(Clone, Debug)]
pub struct SyntheticSequence {
    /// Agent trajectories (groups, attacking directions, pitch, markings).
    pub batch: AgentTrajectories,
    /// Focal (ball) path, one sample per time.
    pub focal: Vec<Point2>,
    /// `(frame, description)` tags for TUI event jumps.
    pub events: Vec<(usize, String)>,
    /// Per-agent planted [`SpaceAction`] over time (script, not classifier).
    pub labels: Vec<Vec<SpaceAction>>,
}

/// 3-minute 11v11 at 1 Hz on the FIFA 105×68 m default.
pub fn generate_11v11_play() -> SyntheticSequence {
    generate_11v11_play_with(PLAY_11V11_DURATION_SEC, PLAY_11V11_DT_SEC)
}

/// 11v11 sequence with explicit duration and `dt` (seconds).
pub fn generate_11v11_play_with(duration: f64, dt: f64) -> SyntheticSequence {
    let dt = dt.max(1e-6);
    let duration = duration.max(dt);
    let (dims, marks) = crate::space::soccer::default_pitch();
    let (xmin, xmax, ymin, ymax) = dims.bounds();
    let n_steps = (duration / dt).floor() as usize + 1;
    let times: Vec<f64> = (0..n_steps).map(|i| i as f64 * dt).collect();

    let g0_roles = g0_433();
    let g1_roles = g1_442();
    let mut positions: Vec<Vec<Point2>> = (0..N_AGENTS)
        .map(|i| {
            let phase = phase_at(0.0);
            let slot = slot_for(i, &g0_roles, &g1_roles, &phase, Point2::origin(), &dims);
            vec![clamp_pitch(slot, xmin, xmax, ymin, ymax)]
        })
        .collect();

    let mut focal: Vec<Point2> = Vec::with_capacity(n_steps);
    let mut labels = vec![vec![SpaceAction::Neutral; n_steps]; N_AGENTS];
    let mut events: Vec<(usize, String)> = Vec::new();
    let passes = pass_list();
    let mut last_event_name = "";

    for step in 0..n_steps {
        let t = times[step];
        let phase = phase_at(t);
        if phase.event != last_event_name {
            events.push((step, phase.event.to_string()));
            last_event_name = phase.event;
        }

        let ball_y = if step == 0 { 0.0 } else { focal[step - 1].y };

        if step > 0 {
            for (i, traj) in positions.iter_mut().enumerate() {
                let prev = traj[step - 1];
                let mut target = slot_for(i, &g0_roles, &g1_roles, &phase, Point2::new(0.0, ball_y), &dims);
                if let Some(extra) = sprint_along(i, t) {
                    target = shift_along(i, target, extra, &dims);
                }
                let dist = prev.distance(target);
                let speed = role_speed(i, dist, sprint_along(i, t).is_some());
                let next = step_toward(prev, target, speed, dt);
                traj.push(clamp_pitch(next + jitter(i, t), xmin, xmax, ymin, ymax));
            }
        }

        let (ball, in_flight) = ball_at(t, dt, &phase, &passes, &positions, step);
        focal.push(clamp_pitch(ball, xmin, xmax, ymin, ymax));

        let planted_carrier = if in_flight { None } else { Some(phase.carrier) };
        for i in 0..N_AGENTS {
            labels[i][step] = plant_label(i, t, &phase, planted_carrier, positions[i][step], focal[step], &dims);
        }
    }

    let groups = [vec![G0; 11], vec![G1; 11]].concat();
    let mut att = vec![Vec2::new(1.0, 0.0); 11];
    att.extend(vec![Vec2::new(-1.0, 0.0); 11]);
    let mut goal_pos = vec![Point2::new(xmax, 0.0); 11];
    goal_pos.extend(vec![Point2::new(xmin, 0.0); 11]);

    let pos_owned = positions;
    let (mut batch, focal) = build_agent_trajectories(times, pos_owned, groups, att, focal, Some(dims), Some(goal_pos));
    batch = batch.with_play_area_markings(marks);

    SyntheticSequence {
        batch,
        focal,
        events,
        labels,
    }
}

#[derive(Clone, Copy)]
struct Role {
    /// Meters from own goal toward the attack when the team is deep.
    along0: f64,
    /// Meters from own goal when the team is high.
    along1: f64,
    /// Resting lateral offset (m), negative = right from the attacking team’s view (+x).
    lat: f64,
}

#[derive(Clone, Copy)]
struct PlayPhase {
    event: &'static str,
    poss: u32,
    g0_line: f64,
    g1_line: f64,
    carrier: usize,
}

#[derive(Clone, Copy)]
struct Pass {
    t: f64,
    from: usize,
    to: usize,
}

fn g0_433() -> [Role; 11] {
    [
        Role {
            along0: 6.0,
            along1: 10.0,
            lat: 0.0,
        }, // GK
        Role {
            along0: 16.0,
            along1: 42.0,
            lat: -24.0,
        }, // RB
        Role {
            along0: 14.0,
            along1: 38.0,
            lat: -8.0,
        }, // RCB
        Role {
            along0: 14.0,
            along1: 38.0,
            lat: 8.0,
        }, // LCB
        Role {
            along0: 16.0,
            along1: 42.0,
            lat: 24.0,
        }, // LB
        Role {
            along0: 26.0,
            along1: 54.0,
            lat: 0.0,
        }, // CDM
        Role {
            along0: 34.0,
            along1: 64.0,
            lat: -12.0,
        }, // RCM
        Role {
            along0: 34.0,
            along1: 64.0,
            lat: 12.0,
        }, // LCM
        Role {
            along0: 48.0,
            along1: 84.0,
            lat: -22.0,
        }, // RW
        Role {
            along0: 52.0,
            along1: 92.0,
            lat: 0.0,
        }, // ST
        Role {
            along0: 48.0,
            along1: 84.0,
            lat: 22.0,
        }, // LW
    ]
}

fn g1_442() -> [Role; 11] {
    [
        Role {
            along0: 6.0,
            along1: 10.0,
            lat: 0.0,
        }, // GK
        Role {
            along0: 16.0,
            along1: 42.0,
            lat: 24.0,
        }, // RB (their right = +y when attacking −x)
        Role {
            along0: 14.0,
            along1: 38.0,
            lat: 8.0,
        }, // RCB
        Role {
            along0: 14.0,
            along1: 38.0,
            lat: -8.0,
        }, // LCB
        Role {
            along0: 16.0,
            along1: 42.0,
            lat: -24.0,
        }, // LB
        Role {
            along0: 32.0,
            along1: 62.0,
            lat: 22.0,
        }, // RM
        Role {
            along0: 30.0,
            along1: 58.0,
            lat: 8.0,
        }, // RCM
        Role {
            along0: 30.0,
            along1: 58.0,
            lat: -8.0,
        }, // LCM
        Role {
            along0: 32.0,
            along1: 62.0,
            lat: -22.0,
        }, // LM
        Role {
            along0: 50.0,
            along1: 88.0,
            lat: 8.0,
        }, // RST
        Role {
            along0: 50.0,
            along1: 88.0,
            lat: -8.0,
        }, // LST
    ]
}

fn phase_at(t: f64) -> PlayPhase {
    match t {
        t if t < 20.0 => PlayPhase {
            event: "build-up",
            poss: G0,
            g0_line: 0.32,
            g1_line: 0.38,
            carrier: 5,
        },
        t if t < 36.0 => PlayPhase {
            event: "progress",
            poss: G0,
            g0_line: 0.50,
            g1_line: 0.40,
            carrier: 6,
        },
        t if t < 48.0 => PlayPhase {
            event: "attacking third",
            poss: G0,
            g0_line: 0.68,
            g1_line: 0.28,
            carrier: 9,
        },
        t if t < 58.0 => PlayPhase {
            event: "wide combination",
            poss: G0,
            g0_line: 0.72,
            g1_line: 0.26,
            carrier: 8,
        },
        t if t < 69.0 => PlayPhase {
            event: "creation",
            poss: G0,
            g0_line: 0.80,
            g1_line: 0.22,
            carrier: 9,
        },
        t if t < 86.0 => PlayPhase {
            event: "counter",
            poss: G1,
            g0_line: 0.22,
            g1_line: 0.70,
            carrier: if t < 78.0 { 13 } else { 20 },
        },
        t if t < 110.0 => PlayPhase {
            event: "G1 attack",
            poss: G1,
            g0_line: 0.24,
            g1_line: 0.62,
            carrier: 20,
        },
        t if t < 126.0 => PlayPhase {
            event: "G1 left",
            poss: G1,
            g0_line: 0.22,
            g1_line: 0.66,
            carrier: 21,
        },
        t if t < 151.0 => PlayPhase {
            event: "switch play",
            poss: G0,
            g0_line: 0.48,
            g1_line: 0.40,
            carrier: if t < 140.0 { 4 } else { 8 },
        },
        t if t < 166.0 => PlayPhase {
            event: "late creation",
            poss: G0,
            g0_line: 0.78,
            g1_line: 0.20,
            carrier: 9,
        },
        _ => PlayPhase {
            event: "reset",
            poss: G0,
            g0_line: 0.30,
            g1_line: 0.32,
            carrier: 5,
        },
    }
}

fn pass_list() -> Vec<Pass> {
    vec![
        Pass {
            t: 20.0,
            from: 5,
            to: 6,
        },
        Pass {
            t: 35.0,
            from: 6,
            to: 9,
        },
        Pass {
            t: 48.0,
            from: 9,
            to: 8,
        },
        Pass {
            t: 58.0,
            from: 8,
            to: 9,
        },
        Pass {
            t: 69.0,
            from: 9,
            to: 13,
        },
        Pass {
            t: 78.0,
            from: 13,
            to: 20,
        },
        Pass {
            t: 110.0,
            from: 20,
            to: 21,
        },
        Pass {
            t: 126.0,
            from: 21,
            to: 4,
        },
        Pass {
            t: 140.0,
            from: 4,
            to: 8,
        },
        Pass {
            t: 151.0,
            from: 8,
            to: 9,
        },
    ]
}

fn group_of(i: usize) -> u32 {
    if i < 11 { G0 } else { G1 }
}

fn is_gk(i: usize) -> bool {
    i == 0 || i == 11
}

fn is_wide(i: usize) -> bool {
    matches!(i, 1 | 4 | 8 | 10 | 12 | 15 | 16 | 19)
}

fn slot_for(
    i: usize,
    g0: &[Role; 11],
    g1: &[Role; 11],
    phase: &PlayPhase,
    ball: Point2,
    dims: &PlayingDimensions,
) -> Point2 {
    let (xmin, xmax, ymin, ymax) = dims.bounds();
    let (role, line, own_x, sign) = if i < 11 {
        (g0[i], phase.g0_line, xmin, 1.0)
    } else {
        (g1[i - 11], phase.g1_line, xmax, -1.0)
    };
    let along = role.along0 + (role.along1 - role.along0) * line.clamp(0.0, 1.0);
    let lat_scale = 0.62 + 0.38 * line;
    let y = role.lat * lat_scale + 0.22 * ball.y;
    let x = own_x + sign * along;
    clamp_pitch(Point2::new(x, y), xmin, xmax, ymin, ymax)
}

fn shift_along(i: usize, p: Point2, extra: f64, dims: &PlayingDimensions) -> Point2 {
    let (xmin, xmax, ymin, ymax) = dims.bounds();
    let sign = if i < 11 { 1.0 } else { -1.0 };
    clamp_pitch(Point2::new(p.x + sign * extra, p.y), xmin, xmax, ymin, ymax)
}

/// Extra meters of along-pitch push for scripted attacking runs.
fn sprint_along(i: usize, t: f64) -> Option<f64> {
    match (i, t) {
        (8, t) if (52.0..64.0).contains(&t) => Some(10.0),
        (9, t) if (58.0..69.0).contains(&t) => Some(12.0),
        (20, t) if (78.0..92.0).contains(&t) => Some(12.0),
        (9, t) if (155.0..166.0).contains(&t) => Some(12.0),
        (21, t) if (110.0..124.0).contains(&t) => Some(8.0),
        _ => None,
    }
}

fn role_speed(i: usize, dist: f64, sprint: bool) -> f64 {
    if is_gk(i) {
        return if dist > 4.0 { 3.0 } else { 1.4 };
    }
    if sprint {
        8.0
    } else if dist > 16.0 {
        6.4
    } else if dist > 5.0 {
        3.6
    } else {
        1.6
    }
}

fn step_toward(from: Point2, to: Point2, speed: f64, dt: f64) -> Point2 {
    let d = to - from;
    let n = d.norm();
    if n < 1e-6 {
        from
    } else {
        from + d.normalize() * (speed * dt).min(n)
    }
}

fn jitter(i: usize, t: f64) -> Vec2 {
    Vec2::new(
        0.45 * (0.19 * t + i as f64 * 0.63).sin(),
        0.70 * (0.15 * t + i as f64 * 1.07).cos(),
    )
}

fn clamp_pitch(p: Point2, xmin: f64, xmax: f64, ymin: f64, ymax: f64) -> Point2 {
    Point2::new(p.x.clamp(xmin + 1.0, xmax - 1.0), p.y.clamp(ymin + 1.0, ymax - 1.0))
}

fn ball_at(
    t: f64,
    dt: f64,
    phase: &PlayPhase,
    passes: &[Pass],
    positions: &[Vec<Point2>],
    step: usize,
) -> (Point2, bool) {
    let idx = step.min(positions[phase.carrier].len().saturating_sub(1));
    let carrier_pos = positions[phase.carrier][idx];
    for p in passes {
        // Flight occupies (t_pass, t_pass + 1 s].
        if t > p.t && t <= p.t + 1.0 {
            let from_idx = ((p.t / dt).round() as usize).min(positions[p.from].len().saturating_sub(1));
            let a = positions[p.from][from_idx];
            let b = positions[p.to][idx.min(positions[p.to].len().saturating_sub(1))];
            let u = ((t - p.t) / 1.0).clamp(0.0, 1.0);
            let mid = Point2::new(a.x + (b.x - a.x) * u, a.y + (b.y - a.y) * u);
            return (mid, true);
        }
    }
    let wobble = Vec2::new(0.25, 0.12) * (0.4 * t).sin();
    (carrier_pos + wobble, false)
}

fn plant_label(
    agent: usize,
    t: f64,
    phase: &PlayPhase,
    planted_carrier: Option<usize>,
    pos: Point2,
    ball: Point2,
    dims: &PlayingDimensions,
) -> SpaceAction {
    let (xmin, xmax, _, _) = dims.bounds();
    let own_goal = if agent < 11 {
        Point2::new(xmin, 0.0)
    } else {
        Point2::new(xmax, 0.0)
    };
    let att_goal = if agent < 11 {
        Point2::new(xmax, 0.0)
    } else {
        Point2::new(xmin, 0.0)
    };
    let d_ball = pos.distance(ball);
    let d_att = pos.distance(att_goal);
    let d_own = pos.distance(own_goal);
    let ball_near_own = ball.distance(own_goal) < 32.0;
    let same = group_of(agent) == phase.poss;

    // One planted conversion: G0 striker, end of the first box entry.
    if (t - 68.0).abs() < 0.51 && agent == 9 {
        return SpaceAction::Conversion;
    }
    if phase.event == "reset" {
        return SpaceAction::Neutral;
    }
    if is_gk(agent) && same {
        return SpaceAction::Neutral;
    }

    if same {
        if planted_carrier == Some(agent) {
            return match phase.event {
                "build-up" | "switch play" => SpaceAction::Expansion,
                "progress" | "G1 attack" | "G1 left" | "counter" => SpaceAction::Penetration,
                "attacking third" | "wide combination" | "creation" | "late creation" => {
                    if d_att < 20.0 {
                        SpaceAction::Creation
                    } else {
                        SpaceAction::Penetration
                    }
                }
                _ => SpaceAction::Penetration,
            };
        }
        let creating = matches!(phase.event, "creation" | "late creation" | "attacking third");
        if creating && !is_gk(agent) && d_att < 22.0 && d_ball < 28.0 {
            SpaceAction::Creation
        } else if is_wide(agent) {
            // Far wingers stretching still count as Expansion — the classifier
            // will miss them when they sit outside proximity_radius.
            SpaceAction::Expansion
        } else if d_ball < 18.0 {
            SpaceAction::Penetration
        } else {
            SpaceAction::Expansion
        }
    } else if d_ball < 8.0 {
        SpaceAction::Pressure
    } else if ball_near_own && d_own < 30.0 {
        SpaceAction::Prevention
    } else if is_gk(agent) {
        SpaceAction::Neutral
    } else {
        SpaceAction::Denial
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{
        convex_hull,
        local_triangles,
        polygon_area,
    };

    #[test]
    fn eleven_v_eleven_shape_and_labels() {
        let seq = generate_11v11_play();
        assert_eq!(seq.batch.num_agents(), 22);
        assert_eq!(seq.batch.num_times(), 181);
        assert_eq!(seq.focal.len(), 181);
        assert_eq!(seq.labels.len(), 22);
        assert!(seq.labels.iter().all(|row| row.len() == 181));
        assert_eq!(
            seq.batch.groups.as_deref(),
            Some(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1][..])
        );
        assert!(seq.batch.play_area_markings.is_some());
        assert!(seq.events.len() >= 6);
        // Script plants every non-Neutral action we care about.
        let used: std::collections::HashSet<_> = seq.labels.iter().flatten().copied().collect();
        for need in [
            SpaceAction::Expansion,
            SpaceAction::Penetration,
            SpaceAction::Denial,
            SpaceAction::Pressure,
            SpaceAction::Creation,
            SpaceAction::Conversion,
            SpaceAction::Prevention,
            SpaceAction::Neutral,
        ] {
            assert!(used.contains(&need), "missing planted {need:?}");
        }
        let players = seq.batch.per_player_summaries(0.8, 1.0, Some(&seq.focal));
        assert_eq!(players.len(), 22);
        assert!(players.iter().all(|p| p.total_distance > 0.0));
        let groups = seq.batch.per_group_summaries(0.8, 1.0, Some(&seq.focal));
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].num_players, 11);
        assert_eq!(groups[1].num_players, 11);
    }

    #[test]
    fn defending_hull_and_attacking_triangles_at_creation() {
        let seq = generate_11v11_play();
        // t=60 s is the G0 creation phase (carrier 9).
        let idx = 60;
        let frame = seq.batch.frame(idx).expect("frame");
        let g1: Vec<Point2> = frame
            .agent_positions
            .iter()
            .enumerate()
            .filter(|(i, _)| *i >= 11)
            .map(|(_, p)| *p)
            .collect();
        let g0: Vec<Point2> = frame.agent_positions.iter().copied().take(11).collect();
        let hull = convex_hull(&g1);
        assert!(hull.len() >= 3);
        let area = polygon_area(&hull);
        assert!(area > 200.0 && area < 6_000.0, "hull area {area}");
        let tris = local_triangles(&g0, seq.focal[idx], 6, 30.0);
        assert!(!tris.is_empty(), "expected local attacking triangles near the ball");
    }

    #[test]
    fn custom_dt_changes_frame_count() {
        let seq = generate_11v11_play_with(12.0, 1.0);
        assert_eq!(seq.batch.num_times(), 13);
        assert_eq!(seq.batch.num_agents(), 22);
    }
}
