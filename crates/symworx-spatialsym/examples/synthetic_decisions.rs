use symworx_spatialsym::{
    AgentTrajectories,
    PlayingDimensions,
    Point2,
    Vec2,
};

fn main() {
    println!("symworx-spatialsym synthetic demo\n");
    println!("Demonstrates longer sequences with Creation / Conversion / Prevention (goal-scoring opportunity actions).\n");

    let dt = 0.1;
    let duration = 3.5;  // longer for more interesting sequences including creation/denial/conversion

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

    // 3: event-driven - longer sequence with goal-scoring creation + denial
    // Attacker (agent 1) makes a run to create a scoring opportunity near the target (goal area).
    // Passer (0) plays the ball into space (Creation for receiver).
    // Defender (2) closes to deny the space (Prevention).
    // Later sequence leads toward a conversion.
    let init = vec![
        Point2::new(0., 0.),
        Point2::new(1.2, 2.5),
        Point2::new(0.7, -0.5),
    ];
    let goal_target = Point2::new(8.5, 2.5); // goal area
    let evs = vec![
        // Early run to create space near goal (Creation opportunity)
        symworx_spatialsym::SpatialEvent::StartRun {
            agent: 1,
            target: goal_target,
            speed: 5.5,
            start_time: 0.4,
        },
        symworx_spatialsym::SpatialEvent::Pass {
            from: 0,
            to: 1,
            time: 1.1,
        },
        // Defender closes the space created (Denial / Prevention)
        symworx_spatialsym::SpatialEvent::Close {
            agent: 2,
            target: 1,
            speed: 6.0,
            start_time: 1.3,
        },
        // Second wave: attacker continues or new run toward goal
        symworx_spatialsym::SpatialEvent::StartRun {
            agent: 1,
            target: Point2::new(9.0, 2.8),
            speed: 4.8,
            start_time: 2.0,
        },
        // Another pass / support run creating more danger
        symworx_spatialsym::SpatialEvent::Pass {
            from: 0,
            to: 1,
            time: 2.4,
        },
    ];
    let (ev_t, ev_p, ev_f) =
        symworx_spatialsym::generate_event_driven(init, Point2::new(0.25, 0.), evs, duration, dt);

    let groups = vec![0u32, 0, 1];
    let att = vec![Vec2::new(1., 0.), Vec2::new(1., 0.), Vec2::new(-1., 0.)];
    let dims = Some(symworx_spatialsym::PlayingDimensions::new(105.0, 68.0)); // standard field size
    // Goal positions per agent (their attacking goal). Team 0 attacks right goal, team 1 left.
    let goal_pos = vec![
        Point2::new(52.5, 0.0),   // right goal for attacking team (group 0)
        Point2::new(52.5, 0.0),
        Point2::new(-52.5, 0.0),  // left goal for defending team (group 1)
    ];
    let (batch, focal) =
        symworx_spatialsym::build_agent_trajectories(ev_t, ev_p, groups, att, ev_f, dims, Some(goal_pos));

    println!(
        "\nBatch (3): {} agents, {} times (longer example with explicit goal-scoring creation + denial)",
        batch.num_agents(),
        batch.num_times()
    );

    if let Some(c) = batch.infer_ball_carrier_at(8, None) {
        println!("Inferred carrier around t=0.8: agent {}", c);
    }

    let decs = batch.classify_with_focal_and_params(&focal, 0.5, 10.0, 0.8);
    // Print a wider window of decisions for the key attacker (agent 1) to see Creation/Conversion
    let start = 5;
    let end = (batch.num_times()).min(28);
    println!(
        "\nAgent1 decisions (t={}..{}): {:?}",
        start,
        end,
        decs[1][start..end].iter().map(|d| d.action).collect::<Vec<_>>()
    );

    // Also show defender decisions (agent 2) to see Prevention/Denial
    println!(
        "Agent2 (defender) decisions (t={}..{}): {:?}",
        start,
        end,
        decs[2][start..end].iter().map(|d| d.action).collect::<Vec<_>>()
    );

    let sums = batch.per_player_summaries(2.0, 4.5, Some(&focal));
    println!(
        "\nPer-player loads: {:?}",
        sums.iter()
            .map(|s| (s.player_idx, s.estimated_load.round()))
            .collect::<Vec<_>>()
    );

    // Ground truth using the new longer "scoring_sequence" scenario (Creation + Conversion + Prevention)
    println!("\n=== Longer scoring sequence demo (Creation + Conversion + Prevention) ===");
    let n_steps = batch.num_times();
    let seq_labels = symworx_spatialsym::generate_ground_truth(3, n_steps, "scoring_sequence");

    // Show classifier output over the key window
    let key_start = 6;
    let key_end = n_steps.min(26);
    println!(
        "Classifier (agent0 creator) t={}-{}: {:?}",
        key_start, key_end,
        decs[0][key_start..key_end].iter().map(|d| d.action).collect::<Vec<_>>()
    );
    println!(
        "Classifier (agent1) t={}-{}: {:?}",
        key_start, key_end,
        decs[1][key_start..key_end].iter().map(|d| d.action).collect::<Vec<_>>()
    );
    println!(
        "Classifier (agent2 denier) t={}-{}: {:?}",
        key_start, key_end,
        decs[2][key_start..key_end].iter().map(|d| d.action).collect::<Vec<_>>()
    );

    println!("\nGround truth labels for scoring_sequence:");
    println!(
        "Agent 0 (creator / Creation phase): {:?}",
        seq_labels[0][key_start..key_end].iter().map(|a| format!("{:?}", a)).collect::<Vec<_>>()
    );
    println!(
        "Agent 1 (Conversion / finishing):   {:?}",
        seq_labels[1][key_start..key_end].iter().map(|a| format!("{:?}", a)).collect::<Vec<_>>()
    );
    println!(
        "Agent 2 (Prevention / Denial):      {:?}",
        seq_labels[2][key_start..key_end].iter().map(|a| format!("{:?}", a)).collect::<Vec<_>>()
    );

    println!("\nSynthetic (1-3) generated + analyzed. Ready for testing/visualization.");
}
