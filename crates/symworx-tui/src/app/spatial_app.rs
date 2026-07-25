// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! SpatialSym demo seed and CSV load.

use std::path::PathBuf;

use symworx_spatialsym::{
    decision::{
        AgentDecision,
        SpaceAction,
    },
    synthetic,
    AgentTrajectories,
    PlayingDimensions,
    Point2,
    Vec2,
};

use super::{
    App,
    SpatialView,
    Tab,
    Workflow,
};

impl App {
    pub fn seed_spatial_demo(&mut self) {
        use symworx_spatialsym::Point2;
        let init = vec![
            Point2::new(0., 0.),
            Point2::new(1.2, 2.5),
            Point2::new(0.7, -0.5),
        ];
        let evs = vec![
            synthetic::SpatialEvent::StartRun {
                agent: 1,
                target: Point2::new(6.3, 2.5),
                speed: 4.0,
                start_time: 0.2,
            },
            synthetic::SpatialEvent::Pass {
                from: 0,
                to: 1,
                time: 0.6,
            },
            synthetic::SpatialEvent::Close {
                agent: 2,
                target: 1,
                speed: 5.2,
                start_time: 0.7,
            },
        ];
        let (ev_t, ev_p, ev_f) =
            synthetic::generate_event_driven(init, Point2::new(0.25, 0.), &evs, 1.4, 0.1);
        let groups = vec![0u32, 0, 1];
        let att = vec![Vec2::new(1., 0.), Vec2::new(1., 0.), Vec2::new(-1., 0.)];
        let dims = Some(PlayingDimensions::new(105.0, 68.0));
        let goal_pos = vec![
            Point2::new(52.5, 0.0),
            Point2::new(52.5, 0.0),
            Point2::new(-52.5, 0.0),
        ];
        let (batch, focal) = symworx_spatialsym::build_agent_trajectories(
            ev_t.clone(),
            ev_p,
            groups,
            att,
            ev_f.clone(),
            dims,
            Some(goal_pos),
        );
        let n_steps = batch.num_times();
        self.spatial_batch = Some(batch);
        self.spatial_focal = Some(focal);
        self.spatial_frame_idx = 0;
        self.spatial_labels = Some(symworx_spatialsym::synthetic::generate_ground_truth(
            3,
            n_steps,
            "pass_then_press",
        ));
        // Wire classifier decisions so conf / features (spd, fwd, near, dfoc, etc) are populated
        if let (Some(b), Some(foc)) = (&self.spatial_batch, &self.spatial_focal) {
            let decs = b.classify_with_focal_and_params(foc, 0.5, 10.0, 0.8);
            self.spatial_decisions = Some(decs);
        }
        // Seed a few event markers for < > / digit nav (based on synthetic event times ~0.2/0.6/0.7s @dt=0.1)
        self.spatial_events = vec![
            (0, "start".to_string()),
            (2, "run".to_string()),
            (6, "pass".to_string()),
            (7, "close".to_string()),
        ];
    }
    pub fn load_spatial_csv(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        use symworx_spatialsym::{
            build_agent_trajectories,
            PlayingDimensions,
            Point2,
        };
        let path_str = path.to_string_lossy().to_string();
        let (times, trajs) = symworx_spatialsym::load_trajectories_csv(&path_str)
            .map_err(|e| anyhow::anyhow!("spatial load: {}", e))?;

        if trajs.is_empty() {
            anyhow::bail!("no trajectories in spatial csv");
        }

        // Build minimal synthetic-like structures so existing viz + summaries work.
        let n_agents = trajs.len();
        let n_steps = times.len().min(trajs[0].len());

        // Trim trajs to common length
        let trimmed: Vec<Vec<Point2>> = trajs
            .into_iter()
            .map(|mut v| {
                v.truncate(n_steps);
                v
            })
            .collect();

        // Fake groups / att directions / goal for compatibility
        let groups: Vec<u32> = (0..n_agents as u32).collect();
        let att = vec![symworx_spatialsym::Vec2::new(1., 0.); n_agents];
        let dims = Some(PlayingDimensions::new(105.0, 68.0));
        let goal_pos = vec![Point2::new(52.5, 0.0); n_agents];

        let ev_t = times.into_iter().take(n_steps).collect();
        let mut ev_f: Vec<Point2> = Vec::new();
        for t in 0..n_steps {
            let fx = trimmed
                .first()
                .and_then(|v| v.get(t))
                .map(|p| p.x)
                .unwrap_or(0.0);
            ev_f.push(Point2::new(fx + 2.0, 1.0));
        }
        let (batch, focal) =
            build_agent_trajectories(ev_t, trimmed, groups, att, ev_f, dims, Some(goal_pos));
        self.spatial_batch = Some(batch);
        self.spatial_focal = Some(focal);
        self.spatial_frame_idx = 0;

        // Rebuild decisions + labels for viz features
        if let (Some(b), Some(foc)) = (&self.spatial_batch, &self.spatial_focal) {
            let n_t = b.num_times();
            self.spatial_labels = Some(symworx_spatialsym::synthetic::generate_ground_truth(
                n_agents,
                n_t,
                "pass_then_press",
            ));
            let decs = b.classify_with_focal_and_params(foc, 0.5, 10.0, 0.8);
            self.spatial_decisions = Some(decs);
        }
        self.spatial_events = vec![(0, "start".to_string()), (n_steps / 2, "mid".to_string())];
        self.current_tab = Tab::Spatial;
        self.spatial_view = SpatialView::Visualize;
        self.current_workflow = Workflow::SpatialSym;
        self.status = format!(
            "Spatial loaded: {} ({} agents, {} steps)",
            path.display(),
            n_agents,
            n_steps
        );
        self.ensure_status_for_current_tab();
        Ok(())
    }
    pub fn refresh_spatial_list(&mut self) {
        // For now reuse main list + filter awareness; dedicated spatial filter separate.
        // Future: dedicated discovery.
    }
}
