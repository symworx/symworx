// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Simple loaders for spatial trajectory data.
//!
//! Initial support: CSV with columns time, agent_id, x, y (in meters).

use std::collections::BTreeMap;

use csv::ReaderBuilder;
#[cfg(feature = "async")]
use tokio::fs;

use crate::{
    error::{
        Result,
        SpatialError,
    },
    geometry::Point2,
};

/// Load trajectories from a CSV file.
///
/// Expected columns (case-insensitive): `time`, `agent_id`, `x`, `y`.
/// Additional columns are ignored for now.
///
/// Returns (times, per-agent trajectories) sorted by agent_id.
pub fn load_trajectories_csv(path: &str) -> Result<(Vec<f64>, Vec<Vec<Point2>>)> {
    let rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .map_err(|e| SpatialError::InvalidParameter(format!("CSV error: {}", e)))?;

    load_trajectories_from_reader(rdr)
}

/// Async version of `load_trajectories_csv`.
///
/// Uses `tokio::fs` to read the file without blocking the runtime,
/// then parses synchronously. For very large files you may want
/// streaming + spawn_blocking.
#[cfg(feature = "async")]
pub async fn load_trajectories_csv_async(path: &str) -> Result<(Vec<f64>, Vec<Vec<Point2>>)> {
    let content = fs::read_to_string(path)
        .await
        .map_err(|e| SpatialError::InvalidParameter(format!("async read failed: {}", e)))?;

    // Reuse the sync parser on the in-memory content
    load_trajectories_from_reader(
        csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(content.as_bytes()),
    )
}

/// Internal helper that takes a csv::Reader (used by both sync and async paths).
fn load_trajectories_from_reader<R: std::io::Read>(mut rdr: csv::Reader<R>) -> Result<(Vec<f64>, Vec<Vec<Point2>>)> {
    let headers = rdr
        .headers()
        .map_err(|e| SpatialError::InvalidParameter(format!("CSV header error: {}", e)))?
        .clone();

    let find_col = |name: &str| -> Option<usize> { headers.iter().position(|h| h.eq_ignore_ascii_case(name)) };

    let col_time = find_col("time").ok_or_else(|| SpatialError::InvalidParameter("missing 'time' column".into()))?;
    let col_id =
        find_col("agent_id").ok_or_else(|| SpatialError::InvalidParameter("missing 'agent_id' column".into()))?;
    let col_x = find_col("x").ok_or_else(|| SpatialError::InvalidParameter("missing 'x' column".into()))?;
    let col_y = find_col("y").ok_or_else(|| SpatialError::InvalidParameter("missing 'y' column".into()))?;

    let mut by_agent: BTreeMap<i32, Vec<(f64, Point2)>> = BTreeMap::new();

    for result in rdr.records() {
        let record = result.map_err(|e| SpatialError::InvalidParameter(format!("CSV row error: {}", e)))?;

        let t: f64 = record
            .get(col_time)
            .unwrap()
            .parse()
            .map_err(|_| SpatialError::InvalidValue("bad time".into()))?;
        let id: i32 = record
            .get(col_id)
            .unwrap()
            .parse()
            .map_err(|_| SpatialError::InvalidValue("bad agent_id".into()))?;
        let x: f64 = record
            .get(col_x)
            .unwrap()
            .parse()
            .map_err(|_| SpatialError::InvalidValue("bad x".into()))?;
        let y: f64 = record
            .get(col_y)
            .unwrap()
            .parse()
            .map_err(|_| SpatialError::InvalidValue("bad y".into()))?;

        by_agent.entry(id).or_default().push((t, Point2::new(x, y)));
    }

    if by_agent.is_empty() {
        return Err(SpatialError::InsufficientData("no data loaded".into()));
    }

    let times: Vec<f64> = by_agent.values().next().unwrap().iter().map(|(t, _)| *t).collect();

    let mut trajectories = Vec::new();
    for (_id, mut entries) in by_agent {
        entries.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let positions: Vec<Point2> = entries.into_iter().map(|(_, p)| p).collect();
        trajectories.push(positions);
    }

    Ok((times, trajectories))
}
