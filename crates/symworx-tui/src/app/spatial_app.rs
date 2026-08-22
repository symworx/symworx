// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! SpatialSym demo seed and CSV load.

use std::path::PathBuf;

use super::{
    App,
    SpatialView,
    Tab,
    Workflow,
};

impl App {
    pub fn seed_spatial_demo(&mut self) {
        let (batch, focal, events) = symworx_spatialsym::generate_3v3_attack();
        let n_agents = batch.num_agents();
        let n_steps = batch.num_times();
        self.spatial_batch = Some(batch);
        self.spatial_focal = Some(focal);
        self.spatial_frame_idx = 0;
        self.spatial_labels = Some(symworx_spatialsym::synthetic::generate_ground_truth(
            n_agents,
            n_steps,
            "pass_then_press",
        ));
        if let (Some(b), Some(foc)) = (&self.spatial_batch, &self.spatial_focal) {
            let decs = b.classify_with_focal_and_params(foc, 0.5, 10.0, 0.8);
            self.spatial_decisions = Some(decs);
        }
        self.spatial_events = events;
    }
    pub fn load_spatial_csv(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        use symworx_spatialsym::{
            Point2,
            build_agent_trajectories,
        };
        let path_str = path.to_string_lossy().to_string();
        let (times, trajs) =
            symworx_spatialsym::load_trajectories_csv(&path_str).map_err(|e| anyhow::anyhow!("spatial load: {}", e))?;

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
        let (dims_v, marks) = symworx_spatialsym::soccer::default_pitch();
        let dims = Some(dims_v);
        let goal_pos = vec![Point2::new(dims_v.bounds().1, 0.0); n_agents];

        let ev_t = times.into_iter().take(n_steps).collect();
        let mut ev_f: Vec<Point2> = Vec::new();
        for t in 0..n_steps {
            let fx = trimmed.first().and_then(|v| v.get(t)).map(|p| p.x).unwrap_or(0.0);
            ev_f.push(Point2::new(fx + 2.0, 1.0));
        }
        let (mut batch, focal) = build_agent_trajectories(ev_t, trimmed, groups, att, ev_f, dims, Some(goal_pos));
        batch = batch.with_play_area_markings(marks);
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
