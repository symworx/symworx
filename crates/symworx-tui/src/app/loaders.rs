// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! File loading helpers on [`App`].

use std::path::PathBuf;

use super::{
    App,
    ExploreView,
    LoadedSignal,
    PeakDetectParams,
    PendingColumnLoad,
    SignalKind,
    Tab,
    TachogramSource,
    Workflow,
};

impl App {
    pub fn load_selected_or_manual(&mut self) -> anyhow::Result<()> {
        if let Some(path) = self.selected_path().cloned() {
            self.load_file(&path)
        } else if !self.manual_path.is_empty() {
            let path = PathBuf::from(&self.manual_path);
            self.load_file(&path)
        } else {
            anyhow::bail!("no file selected and no manual path")
        }
    }
    pub fn load_file(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        if !path.exists() {
            anyhow::bail!("file does not exist: {}", path.display());
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext == "csv" || ext == "txt" || ext == "dat" {
            return self.load_csv(path);
        }
        if ext == "ibi" {
            return self.load_ibi(path);
        }
        if let Ok(signal) = self.try_load_parquet(path) {
            self.loaded_signal = Some(signal);
            self.current_tab = Tab::Explore;
            self.current_workflow = Workflow::BioSym;
            self.status = format!("Loaded {} (switched to Explore)", path.display());
            self.ensure_status_for_current_tab();
            return Ok(());
        }
        anyhow::bail!("unsupported or failed: {}", path.display())
    }
    pub fn try_load_parquet(&self, _path: &PathBuf) -> anyhow::Result<LoadedSignal> {
        // stub, use simple or assume
        Err(anyhow::anyhow!("parquet not fully wired here"))
    }
    pub fn load_csv(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        use std::{
            fs::File,
            io::{
                BufRead,
                BufReader,
            },
        };
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut series = Vec::new();
        let mut has_header = false;
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if !has_header
                && (trimmed.contains(',') || trimmed.parse::<f64>().is_err()) {
                    has_header = true;
                    continue;
                }

            // Take last column as signal value (supports "time,signal" generated files + headers)
            let parts: Vec<&str> = trimmed
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty())
                .collect();
            if let Some(last) = parts.last() {
                if let Ok(v) = last.parse::<f64>() {
                    series.push(v);
                }
            }
        }
        if series.is_empty() {
            anyhow::bail!("no data");
        }
        let (known_p, known_s) = crate::generate::load_peaks_sidecar(path);
        let kind = SignalKind::from_path(path);
        let fs = kind.default_fs();
        let n_known = known_p.len();
        let n_sec = known_s.len();
        let n = series.len();
        self.loaded_signal = Some(LoadedSignal::with_meta(
            series,
            path.display().to_string(),
            fs,
            kind,
            known_p,
            known_s,
        ));
        if let Some(sig) = self.loaded_signal.as_mut() {
            if n_known >= 2 {
                sig.tachogram_source = TachogramSource::KnownPrimary;
                sig.rebuild_tachogram();
            }
        }
        self.peak_params = PeakDetectParams::for_kind(kind);
        self.peak_param_selection = 0;
        self.pending_peak_params = false;
        self.explore_scroll = 0;
        self.explore_view = ExploreView::Waveform;
        self.current_tab = Tab::Explore;
        self.current_workflow = Workflow::BioSym;
        self.status = if n_known + n_sec > 0 {
            format!(
                "Loaded {} ({} samples, {}) — known {}/{} — Explore  [k detect  i tachogram  e export]",
                path.display(),
                n,
                kind.label(),
                n_known,
                n_sec
            )
        } else {
            format!(
                "Loaded {} ({} samples) — Explore  [k detect  i tachogram  e export]",
                path.display(),
                n
            )
        };
        self.ensure_status_for_current_tab();
        Ok(())
    }
    pub fn load_ibi(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        use std::{
            fs::File,
            io::{
                BufRead,
                BufReader,
            },
        };
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut series = Vec::new();
        for line in reader.lines() {
            let line = line?;
            for tok in line.split_whitespace() {
                if let Ok(v) = tok.parse::<f64>() {
                    series.push(v);
                }
            }
        }
        if series.is_empty() {
            anyhow::bail!("no data");
        }
        let n = series.len();
        self.loaded_signal = Some(LoadedSignal::new(series, path.display().to_string()));
        self.current_tab = Tab::Explore;
        self.current_workflow = Workflow::BioSym;
        self.status = format!("Loaded IBI {} samples — switched to Explore", n);
        self.ensure_status_for_current_tab();
        Ok(())
    }
    pub fn inspect_csv_columns(&self, path: &PathBuf) -> anyhow::Result<usize> {
        use std::{
            fs::File,
            io::{
                BufRead,
                BufReader,
            },
        };
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut first = String::new();
        reader.read_line(&mut first)?;
        Ok(first.trim().split(',').count())
    }
    pub fn try_load_multicolumn(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        let n = self.inspect_csv_columns(path)?;
        if n <= 1 {
            return self.load_csv(path);
        }
        // simple load first col
        self.load_csv(path)?;
        self.status = format!("Multi col ({}), loaded col 0. (full picker later)", n);
        Ok(())
    }
    pub fn enter_column_picker(
        &mut self,
        path: PathBuf,
        data: Vec<Vec<f64>>,
        num_columns: usize,
        headers: Option<Vec<String>>,
    ) -> anyhow::Result<()> {
        self.pending_load = Some(PendingColumnLoad {
            path,
            data,
            columns: num_columns,
            headers,
        });
        self.status = format!("File has {} cols. Press 1-{} ", num_columns, num_columns);
        Ok(())
    }
    pub fn reset_loaded(&mut self) {
        if let Some(s) = &mut self.loaded_signal {
            s.reset();
        }
    }
}
