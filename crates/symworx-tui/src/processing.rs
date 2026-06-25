use crate::app::{App, Tab};
use crate::generate;

pub fn moving_average(data: &[f64], window: usize) -> Vec<f64> {
    if window == 0 { return data.to_vec(); }
    let mut out = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        let start = i.saturating_sub(window / 2);
        let end = (i + window / 2 + 1).min(data.len());
        let sum: f64 = data[start..end].iter().sum();
        out.push(sum / (end - start) as f64);
    }
    out
}

pub fn median_filter(data: &[f64], window: usize) -> Vec<f64> {
    if window == 0 { return data.to_vec(); }
    let mut out = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        let start = i.saturating_sub(window / 2);
        let end = (i + window / 2 + 1).min(data.len());
        let mut w = data[start..end].to_vec();
        w.sort_by(|a, b| a.partial_cmp(b).unwrap());
        out.push(w[w.len() / 2]);
    }
    out
}

pub fn detrend_mean(data: &[f64]) -> Vec<f64> {
    if data.is_empty() { return vec![]; }
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    data.iter().map(|v| v - mean).collect()
}

pub fn generate_demo_and_load(app: &mut App, preset: generate::DemoPreset) -> anyhow::Result<()> {
    let data_dir = std::path::Path::new("data");
    let path = generate::generate_and_save(preset, data_dir)?;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let mut series = Vec::new();
    let mut has_header = false;
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        if !has_header {
            if trimmed.parse::<f64>().is_err() {
                has_header = true;
                continue;
            }
        }
        if let Ok(v) = trimmed.parse::<f64>() {
            series.push(v);
        }
    }
    if series.is_empty() {
        anyhow::bail!("no numeric data in generated file");
    }
    app.loaded_signal = Some(crate::app::LoadedSignal::new(series, path.display().to_string()));
    app.current_tab = Tab::Explore;
    app.status = format!(
        "Generated {} → loaded {} samples. Switched to Explore tab. (Press Ctrl+1 to return to Import)",
        path.display(),
        app.loaded_signal.as_ref().map(|s| s.n_samples).unwrap_or(0)
    );
    app.ensure_status_for_current_tab();
    app.refresh_file_list();
    Ok(())
}
