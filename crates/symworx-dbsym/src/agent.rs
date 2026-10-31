// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Writes for the agentic work log (`agent.sqlite`).
//!
//! v1 stores a title, an attempt summary, and an optional prompt SHA-256.
//! `queries.body` stays null. Callers keep secrets and identifiers out of
//! the text fields; this module stores the strings it is given.

use std::path::{
    Component,
    Path,
};

use rusqlite::{
    Connection,
    OptionalExtension,
    params,
};

use crate::error::{
    DbSymError,
    Result,
};

/// Insert one open query. Returns the new id. There is no prompt-body argument.
pub fn open_query(
    conn: &Connection,
    title: &str,
    parent_id: Option<&str>,
    harness: Option<&str>,
    role: Option<&str>,
    agent: Option<&str>,
    body_sha256: Option<&str>,
) -> Result<String> {
    if title.trim().is_empty() {
        return Err(DbSymError::InvalidParameter("title is required".into()));
    }
    if let Some(hash) = body_sha256 {
        require_sha256("body_sha256", hash)?;
    }
    if let Some(parent) = parent_id {
        require_row(conn, "SELECT 1 FROM queries WHERE id = ?1", parent, "parent query")?;
    }
    conn.query_row(
        "INSERT INTO queries (id, parent_id, harness, role, agent, title, body, body_sha256, status)
         VALUES (lower(hex(randomblob(16))), ?1, ?2, ?3, ?4, ?5, NULL, ?6, 'open')
         RETURNING id",
        params![parent_id, harness, role, agent, title, body_sha256],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

/// Move a query to `done` or `abandoned`. Does not finish its attempts.
pub fn finish_query(conn: &Connection, id: &str, status: &str) -> Result<()> {
    match status {
        "done" | "abandoned" => {}
        other => {
            return Err(DbSymError::InvalidParameter(format!(
                "query status must be done or abandoned (got {other})"
            )));
        }
    }
    let n = conn.execute(
        "UPDATE queries SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![status, id],
    )?;
    if n == 0 {
        return Err(DbSymError::InvalidParameter(format!("query {id} not found")));
    }
    Ok(())
}

/// Start an attempt on an existing query. `kind` defaults to `session`.
pub fn start_attempt(
    conn: &Connection,
    query_id: &str,
    agent: &str,
    kind: Option<&str>,
    parent_attempt_id: Option<&str>,
) -> Result<String> {
    if agent.trim().is_empty() {
        return Err(DbSymError::InvalidParameter("agent is required".into()));
    }
    let kind = match kind.unwrap_or("session") {
        "session" => "session",
        "spawn" => "spawn",
        "revise" => "revise",
        other => {
            return Err(DbSymError::InvalidParameter(format!(
                "unknown kind {other} (use session, spawn, or revise)"
            )));
        }
    };
    require_row(conn, "SELECT 1 FROM queries WHERE id = ?1", query_id, "query")?;
    if let Some(parent) = parent_attempt_id {
        let parent_query: String =
            match conn.query_row("SELECT query_id FROM attempts WHERE id = ?1", params![parent], |row| {
                row.get(0)
            }) {
                Ok(q) => q,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Err(DbSymError::InvalidParameter(format!(
                        "parent attempt {parent} not found"
                    )));
                }
                Err(e) => return Err(e.into()),
            };
        if parent_query != query_id {
            return Err(DbSymError::InvalidParameter(
                "parent attempt belongs to a different query".into(),
            ));
        }
    }
    conn.query_row(
        "INSERT INTO attempts (id, query_id, parent_attempt_id, agent, kind, outcome)
         VALUES (lower(hex(randomblob(16))), ?1, ?2, ?3, ?4, 'in_progress')
         RETURNING id",
        params![query_id, parent_attempt_id, agent, kind],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

/// Finish an attempt. Requires a non-empty summary. Does not change query status.
pub fn finish_attempt(conn: &Connection, id: &str, outcome: &str, summary: &str) -> Result<()> {
    match outcome {
        "succeeded" | "failed" | "superseded" | "refused" => {}
        other => {
            return Err(DbSymError::InvalidParameter(format!(
                "unknown outcome {other} (use succeeded, failed, superseded, or refused)"
            )));
        }
    }
    if summary.trim().is_empty() {
        return Err(DbSymError::InvalidParameter("summary is required".into()));
    }
    let n = conn.execute(
        "UPDATE attempts SET outcome = ?1, summary = ?2, finished_at = datetime('now') WHERE id = ?3",
        params![outcome, summary, id],
    )?;
    if n == 0 {
        return Err(DbSymError::InvalidParameter(format!("attempt {id} not found")));
    }
    Ok(())
}

/// Attach a repo-relative path to an attempt. Returns the new artifact id.
pub fn add_artifact(conn: &Connection, attempt_id: &str, path: &str, role: &str, sha256: Option<&str>) -> Result<i64> {
    match role {
        "input" | "output" | "log" => {}
        other => {
            return Err(DbSymError::InvalidParameter(format!(
                "unknown artifact role {other} (use input, output, or log)"
            )));
        }
    }
    require_relative_path(path)?;
    if let Some(hash) = sha256 {
        require_sha256("sha256", hash)?;
    }
    require_row(conn, "SELECT 1 FROM attempts WHERE id = ?1", attempt_id, "attempt")?;
    conn.query_row(
        "INSERT INTO artifacts (attempt_id, path, role, sha256) VALUES (?1, ?2, ?3, ?4) RETURNING id",
        params![attempt_id, path, role, sha256],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn require_sha256(field: &str, value: &str) -> Result<()> {
    let ok = value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit());
    if ok {
        Ok(())
    } else {
        Err(DbSymError::InvalidParameter(format!("{field} must be 64 hex digits")))
    }
}

fn require_relative_path(path: &str) -> Result<()> {
    if path.trim().is_empty() {
        return Err(DbSymError::InvalidParameter("path is required".into()));
    }
    let parsed = Path::new(path);
    let bad = parsed.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    });
    if bad {
        return Err(DbSymError::InvalidParameter(
            "path must be repo-relative and must not contain '..'".into(),
        ));
    }
    Ok(())
}

fn require_row(conn: &Connection, sql: &str, id: &str, label: &str) -> Result<()> {
    if id.is_empty() {
        return Err(DbSymError::InvalidParameter(format!("{label} id is required")));
    }
    let found: Option<i32> = conn.query_row(sql, params![id], |row| row.get(0)).optional()?;
    if found.is_none() {
        return Err(DbSymError::InvalidParameter(format!("{label} {id} not found")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
    };

    use super::*;
    use crate::{
        StoreProfile,
        init::{
            init,
            open_agent,
        },
    };

    fn scratch() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "symworx-dbsym-agent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&p);
        p
    }

    fn setup() -> (PathBuf, Connection) {
        let root = scratch();
        let report = init(&root, StoreProfile::Agent, &[]).expect("init");
        let conn = open_agent(&report.db).expect("open");
        (root, conn)
    }

    fn sha() -> String {
        "ab".repeat(32)
    }

    #[test]
    fn round_trip_reads_back() {
        let (root, conn) = setup();
        let hash = sha();
        let query = open_query(
            &conn,
            "week 3 lab",
            None,
            Some("education"),
            Some("instructor"),
            Some("instructor"),
            Some(&hash),
        )
        .expect("open query");
        assert_eq!(query.len(), 32);

        let attempt = start_attempt(&conn, &query, "instructor", Some("session"), None).expect("start");
        let spawn = start_attempt(&conn, &query, "reviewer", Some("spawn"), Some(&attempt)).expect("spawn");
        let child = open_query(&conn, "delegated brief", Some(&query), None, None, None, None).expect("child");

        let status: String = conn
            .query_row("SELECT status FROM queries WHERE id = ?1", [&query], |row| row.get(0))
            .unwrap();
        assert_eq!(status, "open");

        finish_attempt(&conn, &attempt, "succeeded", "drafted a slide outline").expect("finish attempt");
        let still_open: String = conn
            .query_row("SELECT status FROM queries WHERE id = ?1", [&query], |row| row.get(0))
            .unwrap();
        assert_eq!(still_open, "open");

        let artifact = add_artifact(&conn, &attempt, "lectures/week3.md", "output", Some(&hash)).expect("artifact");
        assert!(artifact >= 1);
        finish_query(&conn, &query, "done").expect("finish query");

        let (body, stored_hash, stored_status): (Option<String>, String, String) = conn
            .query_row(
                "SELECT body, body_sha256, status FROM queries WHERE id = ?1",
                [&query],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert!(body.is_none());
        assert_eq!(stored_hash, hash);
        assert_eq!(stored_status, "done");

        let (summary, outcome, finished): (String, String, Option<String>) = conn
            .query_row(
                "SELECT summary, outcome, finished_at FROM attempts WHERE id = ?1",
                [&attempt],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(summary, "drafted a slide outline");
        assert_eq!(outcome, "succeeded");
        assert!(finished.is_some());

        let (path, role): (String, String) = conn
            .query_row("SELECT path, role FROM artifacts WHERE id = ?1", [artifact], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(path, "lectures/week3.md");
        assert_eq!(role, "output");

        let parent: String = conn
            .query_row("SELECT parent_id FROM queries WHERE id = ?1", [&child], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(parent, query);
        let spawn_kind: String = conn
            .query_row("SELECT kind FROM attempts WHERE id = ?1", [&spawn], |row| row.get(0))
            .unwrap();
        assert_eq!(spawn_kind, "spawn");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_bad_input() {
        let (root, conn) = setup();
        let query = open_query(&conn, "one", None, None, None, None, None).expect("q");
        let other = open_query(&conn, "two", None, None, None, None, None).expect("q2");
        let attempt = start_attempt(&conn, &query, "researcher", None, None).expect("a");

        let empty = open_query(&conn, "", None, None, None, None, None).unwrap_err();
        assert!(empty.to_string().contains("title"), "{empty}");

        let bad_hash = open_query(&conn, "t", None, None, None, None, Some("abcd")).unwrap_err();
        assert!(bad_hash.to_string().contains("64 hex"), "{bad_hash}");

        let empty_summary = finish_attempt(&conn, &attempt, "failed", "  ").unwrap_err();
        assert!(empty_summary.to_string().contains("summary"), "{empty_summary}");

        let unknown = finish_attempt(&conn, &attempt, "nope", "tried").unwrap_err();
        assert!(unknown.to_string().contains("outcome"), "{unknown}");

        let absolute = add_artifact(&conn, &attempt, "/etc/passwd", "input", None).unwrap_err();
        assert!(absolute.to_string().contains("repo-relative"), "{absolute}");

        let parent_seg = add_artifact(&conn, &attempt, "foo/../../etc", "log", None).unwrap_err();
        assert!(parent_seg.to_string().contains("repo-relative"), "{parent_seg}");

        let crossed = start_attempt(&conn, &other, "researcher", None, Some(&attempt)).unwrap_err();
        assert!(crossed.to_string().contains("different query"), "{crossed}");

        let missing = start_attempt(&conn, "missing-query", "researcher", None, None).unwrap_err();
        assert!(missing.to_string().contains("not found"), "{missing}");

        let _ = fs::remove_dir_all(&root);
    }
}
