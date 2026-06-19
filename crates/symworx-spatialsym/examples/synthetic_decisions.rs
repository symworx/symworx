use symworx_spatialsym::{
    AgentTrajectories,
    ArenaSpec,
    Point2,
    Vec2,
};

fn main() {
    println!("symworx-spatialsym synthetic demo (options 1-3)\n");

    let dt = 0.1;
    let duration = 1.4;

    // 1+2: parametric + noisy (math)
    let _t: Vec<f64> = (0..=((duration / dt) as usize))
        .map(|i| i as f64 * dt)
        .collect();
    let lin = symworx_spatialsym::generate_linear_trajectory(
        Point2::new(0., 0.),
        Vec2::new(4., 0.),
        duration,
        dt,
    );
    let curved = symworx_spatialsym::generate_curved_trajectory(
        Point2::new(0., 2.),
        Vec2::new(3.5, 0.),
        duration,
        dt,
        1.0,
        3.5,
    );
    let noisy = symworx_spatialsym::generate_noisy_trajectory(
        Point2::new(0., 4.),
        Vec2::new(3.8, 0.),
        duration,
        dt,
        0.3,
    );
    println!(
        "1+2: lin={} curved={} noisy={}",
        lin.len(),
        curved.len(),
        noisy.len()
    );

    // 3: event-driven
    let init = vec![
        Point2::new(0., 0.),
        Point2::new(1.2, 2.5),
        Point2::new(0.7, -0.5),
    ];
    let evs = vec![
        symworx_spatialsym::SpatialEvent::StartRun {
            agent: 1,
            target: Point2::new(6.3, 2.5),
            speed: 4.0,
            start_time: 0.2,
        },
        symworx_spatialsym::SpatialEvent::Pass {
            from: 0,
            to: 1,
            time: 0.6,
        },
        symworx_spatialsym::SpatialEvent::Close {
            agent: 2,
            target: 1,
            speed: 5.2,
            start_time: 0.7,
        },
    ];
    let (ev_t, ev_p, ev_f) =
        symworx_spatialsym::generate_event_driven(init, Point2::new(0.25, 0.), evs, duration, dt);

    let groups = vec![0u32, 0, 1];
    let att = vec![Vec2::new(1., 0.), Vec2::new(1., 0.), Vec2::new(-1., 0.)];
    let (batch, focal) =
        symworx_spatialsym::build_agent_trajectories(ev_t, ev_p, groups, att, ev_f);

    println!(
        "\nBatch (3): {} agents, {} times",
        batch.num_agents(),
        batch.num_times()
    );

    if let Some(c) = batch.infer_ball_carrier_at(3, None) {
        println!("Inferred carrier ~t=0.3: agent {}", c);
    }

    let decs = batch.classify_with_focal_and_params(&focal, 0.35, 9.0, 0.6);
    println!(
        "Agent1 mid decs: {:?}",
        decs[1][3..8].iter().map(|d| d.action).collect::<Vec<_>>()
    );

    let sums = batch.per_player_summaries(2.0, 4.5, Some(&focal));
    println!(
        "Per-player loads: {:?}",
        sums.iter()
            .map(|s| (s.player_idx, s.estimated_load.round()))
            .collect::<Vec<_>>()
    );

    println!("\nSynthetic (1-3) generated + analyzed. Ready for testing/visualization.");
}
