// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Throwaway local demo: HTTP on loopback, synthetic rows into a dbSym catalog.
//!
//! Not an API. Not for production. Ingest still belongs in the library later.
//!
//! ```bash
//! cargo run -p symworx-dbsym --example local_server --features sqlite
//! # browser or: curl -s http://127.0.0.1:8765/
//! #              curl -s 'http://127.0.0.1:8765/generate?n=3'
//! ```
//!
//! `--dir` defaults to a temp folder. `--port` default 8765. Ctrl+C to stop.

use std::{
    env,
    fs,
    io::{
        BufRead,
        BufReader,
        Write,
    },
    net::{
        TcpListener,
        TcpStream,
    },
    path::{
        Path,
        PathBuf,
    },
};

use rusqlite::Connection;
use symworx_dbsym::{
    DEFAULT_DB_RELATIVE,
    DEFAULT_OBJECTS_RELATIVE,
    StoreProfile,
    init::{
        init,
        open,
    },
};

fn main() {
    let mut port = 8765u16;
    let mut dir = env::temp_dir().join("symworx-dbsym-demo");
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" => {
                port = args.next().and_then(|s| s.parse().ok()).unwrap_or(port);
            }
            "--dir" => {
                if let Some(p) = args.next() {
                    dir = PathBuf::from(p);
                }
            }
            "-h" | "--help" => {
                eprintln!(
                    "throwaway demo — not a public API\n\
                     Usage: local_server [--port 8765] [--dir DIR]\n\
                     GET /  GET /status  GET /generate?n=1"
                );
                return;
            }
            other => {
                eprintln!("unknown arg: {other} (try --help)");
                std::process::exit(2);
            }
        }
    }

    let report = init(&dir, StoreProfile::Study, &["exercise_science".to_string()]).expect("init catalog");
    eprintln!("throwaway dbSym demo — not a public API");
    eprintln!("db   {}", report.db.display());
    eprintln!("http http://127.0.0.1:{port}/");
    eprintln!("     GET /generate?n=3   to insert fake subjects");
    eprintln!("Ctrl+C to stop");

    let listener = TcpListener::bind(("127.0.0.1", port)).unwrap_or_else(|e| {
        eprintln!("bind 127.0.0.1:{port}: {e}");
        std::process::exit(1);
    });
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                if let Err(e) = handle(stream, &report.root) {
                    eprintln!("request: {e}");
                }
            }
            Err(e) => eprintln!("accept: {e}"),
        }
    }
}

fn handle(mut stream: TcpStream, root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut first = String::new();
    reader.read_line(&mut first)?;
    let mut buf = String::new();
    loop {
        buf.clear();
        let n = reader.read_line(&mut buf)?;
        if n == 0 || buf == "\r\n" || buf == "\n" {
            break;
        }
    }

    let parts: Vec<&str> = first.split_whitespace().collect();
    let method = parts.first().copied().unwrap_or("");
    let target = parts.get(1).copied().unwrap_or("/");
    let (path, query) = split_query(target);

    if method != "GET" && method != "HEAD" && method != "POST" {
        write_http(&mut stream, 405, "text/plain; charset=utf-8", b"use GET or POST\n")?;
        return Ok(());
    }

    match path {
        "/" | "/index.html" => {
            let body = index_page(root);
            write_http(&mut stream, 200, "text/html; charset=utf-8", body.as_bytes())?;
        }
        "/status" => {
            let body = status_text(root)?;
            write_http(&mut stream, 200, "text/plain; charset=utf-8", body.as_bytes())?;
        }
        "/generate" => {
            let n = query_u32(query, "n").unwrap_or(1).clamp(1, 32);
            let msg = generate(root, n)?;
            if wants_html(query) {
                let body = format!(
                    "<!doctype html><meta charset=utf-8><p>{}</p><p><a href=/>back</a></p>",
                    html_escape(&msg)
                );
                write_http(&mut stream, 200, "text/html; charset=utf-8", body.as_bytes())?;
            } else {
                write_http(&mut stream, 200, "text/plain; charset=utf-8", msg.as_bytes())?;
            }
        }
        _ => {
            write_http(&mut stream, 404, "text/plain; charset=utf-8", b"not found\n")?;
        }
    }
    Ok(())
}

fn split_query(target: &str) -> (&str, &str) {
    match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target, ""),
    }
}

fn query_u32(query: &str, key: &str) -> Option<u32> {
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=')
            && k == key
        {
            return v.parse().ok();
        }
    }
    None
}

fn wants_html(query: &str) -> bool {
    query.split('&').any(|p| p == "html" || p == "html=1")
}

fn write_http(stream: &mut TcpStream, code: u16, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    let reason = match code {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    write!(
        stream,
        "HTTP/1.1 {code} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    Ok(())
}

fn index_page(root: &Path) -> String {
    let status = status_text(root).unwrap_or_else(|e| format!("status error: {e}\n"));
    format!(
        "<!doctype html>
<meta charset=utf-8>
<title>dbSym throwaway demo</title>
<body style=\"font:16px sans-serif;max-width:40rem;margin:2rem\">
<h1>dbSym throwaway demo</h1>
<p>Synthetic data only. Not an API. Catalog under <code>{}</code>.</p>
<pre>{}</pre>
<p><a href=\"/generate?n=1&amp;html=1\">Generate 1 subject</a> ·
   <a href=\"/generate?n=8&amp;html=1\">Generate 8</a> ·
   <a href=\"/status\">status (text)</a></p>
</body>
",
        root.display(),
        html_escape(&status)
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn db_path(root: &Path) -> PathBuf {
    root.join(DEFAULT_DB_RELATIVE)
}

fn status_text(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let conn = open(&db_path(root))?;
    let subjects: i64 = conn.query_row("SELECT COUNT(*) FROM subjects", [], |r| r.get(0))?;
    let sessions: i64 = conn.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?;
    let files: i64 = conn.query_row("SELECT COUNT(*) FROM file_records", [], |r| r.get(0))?;
    let obs: i64 = conn.query_row("SELECT COUNT(*) FROM observations", [], |r| r.get(0))?;
    Ok(format!(
        "root       {}\ndb         {}\nsubjects   {subjects}\nsessions   {sessions}\nfiles      {files}\nobservations {obs}\n",
        root.display(),
        db_path(root).display()
    ))
}

fn attr_id(conn: &Connection, name: &str) -> rusqlite::Result<i64> {
    conn.query_row("SELECT id FROM attributes WHERE name = ?1", [name], |r| r.get(0))
}

fn next_seq(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM subjects", [], |r| r.get(0))
}

fn generate(root: &Path, n: u32) -> Result<String, Box<dyn std::error::Error>> {
    let conn = open(&db_path(root))?;
    let objects = root.join(DEFAULT_OBJECTS_RELATIVE);
    fs::create_dir_all(&objects)?;
    let vo2_id = attr_id(&conn, "vo2_max")?;
    let cond_id = attr_id(&conn, "condition")?;
    let visit_id = attr_id(&conn, "visit")?;

    conn.execute(
        "INSERT INTO ingest_batch (filename, uploader) VALUES ('local_server', 'demo')",
        [],
    )?;
    let batch_id = conn.last_insert_rowid();

    let mut created = Vec::new();
    let base = next_seq(&conn)?;
    for i in 0..n {
        let seq = base + i as i64 + 1;
        let sid = format!("00000000-0000-4000-8000-{seq:012x}");
        let coded = format!("demo{seq:03}");
        conn.execute(
            "INSERT INTO subjects (id, coded_id, ingest_batch_id, status) VALUES (?1, ?2, ?3, 'accepted')",
            rusqlite::params![sid, coded, batch_id],
        )?;

        for (label, kind, condition, date) in [
            ("v1", "baseline", "rest", "2024-01-15"),
            ("v2", "followup", "exercise", "2024-02-08"),
        ] {
            conn.execute(
                "INSERT INTO sessions (subject_id, label, kind, condition, started_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![sid, label, kind, condition, date],
            )?;
            let session_id = conn.last_insert_rowid();

            let fname = format!("{coded}_{label}_{condition}_demo.txt");
            let obj = objects.join(&fname);
            fs::write(&obj, "0.000 1.000\n1.000 1.010\n2.010 0.990\n")?;
            let key = format!("objects/{fname}");
            conn.execute(
                "INSERT INTO file_records
                   (subject_id, session_id, ingest_batch_id, modality, format, object_key, original_name, role)
                 VALUES (?1, ?2, ?3, 'series', 'txt', ?4, ?5, 'raw')",
                rusqlite::params![sid, session_id, batch_id, key, fname],
            )?;

            let vo2 = 40.0 + (seq as f64) + if condition == "exercise" { 5.0 } else { 0.0 };
            conn.execute(
                "INSERT INTO observations (subject_id, session_id, time_s, ingest_batch_id, source_row)
                 VALUES (?1, ?2, 0, ?3, 0)",
                rusqlite::params![sid, session_id, batch_id],
            )?;
            let oid = conn.last_insert_rowid();
            for (aid, val) in [
                (vo2_id, format!("{vo2:.1}")),
                (cond_id, condition.to_string()),
                (visit_id, label.to_string()),
            ] {
                conn.execute(
                    "INSERT INTO observation_values (observation_id, attribute_id, value) VALUES (?1, ?2, ?3)",
                    rusqlite::params![oid, aid, val],
                )?;
            }
        }
        created.push(coded);
    }

    Ok(format!(
        "inserted {} subject(s): {}\n{}",
        created.len(),
        created.join(", "),
        status_text(root)?
    ))
}
