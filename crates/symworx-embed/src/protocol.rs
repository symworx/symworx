// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! JSON-line wire protocol for PPG / vitals streams.
//!
//! # Device line (Arduino / serial)
//!
//! ```json
//! {"red":12345,"ir":23456,"bpm":72.3,"bpm_avg":71,"ts":123456}
//! ```
//!
//! # Host-enriched line (SymWorx)
//!
//! ```json
//! {"sid":"S001","source":"arduino","red":12345,"ir":23456,"bpm":72.3,"bpm_avg":71,"ts":123456}
//! ```
//!
//! ## Naming
//! - **`sid`**: subject id (canonical outbound field).
//! - On parse only: `patient_id` is accepted as an alias for `sid`; `heart_rate` as an alias for `bpm`.
//! - Outbound lines always use `sid` / `bpm` (never the aliases).

use std::time::SystemTime;

use serde_json::Value;

use crate::{
    error::{
        EmbedError,
        Result,
    },
    types::{
        SourceKind,
        StreamSample,
    },
};

/// Parse one JSON object (optionally with surrounding whitespace) into a [`StreamSample`].
///
/// Empty lines return `Ok(None)`. Non-empty but invalid JSON returns [`EmbedError::Protocol`].
pub fn parse_json_line(line: &str) -> Result<Option<StreamSample>> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }

    let v: Value = serde_json::from_str(line).map_err(|e| EmbedError::Protocol(format!("invalid JSON: {e}")))?;

    let obj = v
        .as_object()
        .ok_or_else(|| EmbedError::Protocol("expected JSON object".into()))?;

    let red = int_field(obj.get("red"));
    let ir = int_field(obj.get("ir"));
    let bpm = float_field(obj.get("bpm")).or_else(|| float_field(obj.get("heart_rate")));
    let bpm_avg = float_field(obj.get("bpm_avg"));
    let spo2 = float_field(obj.get("spo2"));
    let resp = float_field(obj.get("resp")).or_else(|| float_field(obj.get("respiration")));
    let device_ts_ms = u64_field(obj.get("ts"));

    // Canonical `sid`; accept `patient_id` as ingress alias only.
    let sid = string_field(obj.get("sid")).or_else(|| string_field(obj.get("patient_id")));

    let source = string_field(obj.get("source"))
        .map(|s| SourceKind::from_tag(&s))
        .unwrap_or(SourceKind::Unknown);

    // Optional host timestamp string (ISO-ish) — best-effort; ignore failures.
    // Keep parsing light: treat presence of timestamp as "now" for host path.
    // Full ISO parsing can be added later without changing the public field.
    let host_ts = string_field(obj.get("timestamp")).map(|_s| SystemTime::now());

    Ok(Some(StreamSample {
        red,
        ir,
        bpm,
        bpm_avg,
        spo2,
        resp,
        device_ts_ms,
        host_ts,
        sid,
        source,
    }))
}

/// Serialize a sample to a compact JSON line (no trailing newline).
///
/// Always emits **`sid`** (never `patient_id`). Omits `None` numeric fields.
pub fn sample_to_json_line(sample: &StreamSample) -> Result<String> {
    let mut map = serde_json::Map::new();

    if let Some(ref sid) = sample.sid {
        map.insert("sid".into(), Value::String(sid.clone()));
    }
    map.insert("source".into(), Value::String(sample.source.as_tag().to_string()));

    insert_i64(&mut map, "red", sample.red);
    insert_i64(&mut map, "ir", sample.ir);
    insert_f64(&mut map, "bpm", sample.bpm);
    insert_f64(&mut map, "bpm_avg", sample.bpm_avg);
    insert_f64(&mut map, "spo2", sample.spo2);
    insert_f64(&mut map, "resp", sample.resp);
    if let Some(ts) = sample.device_ts_ms {
        map.insert("ts".into(), Value::Number(ts.into()));
    }

    serde_json::to_string(&Value::Object(map)).map_err(|e| EmbedError::Protocol(format!("serialize failed: {e}")))
}

/// Enrich a device sample with subject id and source, setting host time if missing.
pub fn enrich(sample: StreamSample, sid: impl Into<String>, source: SourceKind) -> StreamSample {
    let mut s = sample.with_sid(sid).with_source(source);
    if s.host_ts.is_none() {
        s.host_ts = Some(SystemTime::now());
    }
    s
}

fn string_field(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn int_field(v: Option<&Value>) -> Option<i64> {
    match v? {
        Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn u64_field(v: Option<&Value>) -> Option<u64> {
    match v? {
        Value::Number(n) => n.as_u64().or_else(|| n.as_i64().map(|i| i.max(0) as u64)),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn float_field(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn insert_i64(map: &mut serde_json::Map<String, Value>, key: &str, v: Option<i64>) {
    if let Some(n) = v {
        map.insert(key.into(), Value::Number(n.into()));
    }
}

fn insert_f64(map: &mut serde_json::Map<String, Value>, key: &str, v: Option<f64>) {
    if let Some(n) = v
        && let Some(num) = serde_json::Number::from_f64(n)
    {
        map.insert(key.into(), Value::Number(num));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_arduino_line() {
        let line = r#"{"red":12345,"ir":23456,"bpm":72.3,"bpm_avg":71,"ts":123456}"#;
        let s = parse_json_line(line).unwrap().unwrap();
        assert_eq!(s.red, Some(12345));
        assert_eq!(s.ir, Some(23456));
        assert!((s.bpm.unwrap() - 72.3).abs() < 1e-9);
        assert_eq!(s.bpm_avg, Some(71.0));
        assert_eq!(s.device_ts_ms, Some(123456));
        assert!(s.sid.is_none());
    }

    #[test]
    fn parse_patient_id_and_heart_rate_aliases() {
        let line = r#"{"patient_id":"P001","heart_rate":88,"spo2":98,"source":"simulator"}"#;
        let s = parse_json_line(line).unwrap().unwrap();
        assert_eq!(s.sid.as_deref(), Some("P001"));
        assert_eq!(s.bpm, Some(88.0));
        assert_eq!(s.spo2, Some(98.0));
        assert_eq!(s.source, SourceKind::Simulator);
    }

    #[test]
    fn parse_canonical_sid() {
        let line = r#"{"sid":"S001","source":"arduino","ir":100,"bpm":70}"#;
        let s = parse_json_line(line).unwrap().unwrap();
        assert_eq!(s.sid.as_deref(), Some("S001"));
        assert_eq!(s.source, SourceKind::Arduino);
    }

    #[test]
    fn empty_line_is_none() {
        assert!(parse_json_line("  \n").unwrap().is_none());
    }

    #[test]
    fn bad_json_errors() {
        assert!(parse_json_line("not-json").is_err());
    }

    #[test]
    fn serialize_uses_sid_not_patient_id() {
        let s = StreamSample {
            sid: Some("S001".into()),
            source: SourceKind::Arduino,
            red: Some(1),
            ir: Some(2),
            bpm: Some(60.0),
            bpm_avg: Some(61.0),
            spo2: None,
            resp: Some(0.5),
            device_ts_ms: Some(9),
            host_ts: None,
        };
        let line = sample_to_json_line(&s).unwrap();
        assert!(line.contains(r#""sid":"S001""#));
        assert!(!line.contains("patient_id"));
        assert!(line.contains(r#""source":"arduino""#));
    }

    #[test]
    fn enrich_sets_sid_and_source() {
        let raw = parse_json_line(r#"{"ir":1,"bpm":70}"#).unwrap().unwrap();
        let e = enrich(raw, "S002", SourceKind::Arduino);
        assert_eq!(e.sid.as_deref(), Some("S002"));
        assert_eq!(e.source, SourceKind::Arduino);
        assert!(e.host_ts.is_some());
    }
}
