// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! StatsSym helpers on [`App`].

use super::{
    App,
    StatsLabTask,
    StatsView,
};

impl App {
    pub fn smart_stats_lab_cols(table: &symworx_io::TableData) -> (usize, usize) {
        let n = table.n_cols();
        if n == 0 {
            return (0, 0);
        }
        if n == 1 {
            return (0, 0);
        }
        const Y_KEYS: &[&str] = &["y", "target", "label", "response", "outcome", "class"];
        let y = table
            .headers
            .iter()
            .enumerate()
            .find(|(_, h)| {
                let lower = h.to_ascii_lowercase();
                Y_KEYS
                    .iter()
                    .any(|k| lower == *k || lower.ends_with(&format!("_{k}")) || lower.starts_with(&format!("{k}_")))
            })
            .map(|(i, _)| i)
            .unwrap_or(n - 1);
        let x = (0..n).find(|&i| i != y).unwrap_or(0);
        (x, y)
    }
    pub fn stats_col_name(table: &symworx_io::TableData, col: usize) -> String {
        table.headers.get(col).cloned().unwrap_or_else(|| format!("col{col}"))
    }
    pub fn enter_stats_lab_with_table(
        &mut self,
        table: symworx_io::TableData,
        source_msg: impl Into<String>,
        task_hint: Option<StatsLabTask>,
    ) {
        let (x, y) = Self::smart_stats_lab_cols(&table);
        let xname = Self::stats_col_name(&table, x);
        let yname = Self::stats_col_name(&table, y);
        let nrows = table.n_rows();
        let ncols = table.n_cols();

        if let Some(t) = task_hint {
            if let Some(i) = StatsLabTask::ALL.iter().position(|&a| a == t) {
                self.stats_lab_task = i;
            }
        }

        self.stats_lab_x_col = x;
        self.stats_lab_y_col = y;
        self.stats_lab_result = None;
        self.stats_col_focus = 0;
        self.stats_table = Some(table);
        self.stats_view = StatsView::Lab;

        let task_label = StatsLabTask::ALL[self.stats_lab_task.min(StatsLabTask::ALL.len() - 1)].label();
        self.status = format!(
            "{} · {}×{} · X={} Y={} · {} · Enter to run",
            source_msg.into(),
            nrows,
            ncols,
            xname,
            yname,
            task_label
        );
    }
}
