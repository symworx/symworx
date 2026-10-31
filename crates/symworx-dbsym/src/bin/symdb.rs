// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! `symdb` — init a per-study (or edge) SQLite catalog, or an agentic work log.
//!
//! Data files live under `<dir>/.dbsym/` (never in the SymWorx tree).

#[cfg(feature = "sqlite")]
use std::{
    collections::BTreeMap,
    path::PathBuf,
};
use std::{
    env,
    process::ExitCode,
};

use symworx_dbsym::{
    AGENTIC_SCHEMA_VERSION,
    DEFAULT_AGENT_DB_RELATIVE,
    DEFAULT_DB_RELATIVE,
    SCHEMA_VERSION,
};
#[cfg(feature = "sqlite")]
use symworx_dbsym::{
    StoreProfile,
    agent::{
        add_artifact,
        finish_attempt,
        finish_query,
        open_query,
        start_attempt,
    },
    init::{
        apply,
        apply_agent,
        init,
        open_agent,
    },
    seed::available_presets,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return ExitCode::from(1);
    }
    match args[1].as_str() {
        "init" => cmd_init(&args),
        "apply" => cmd_apply(&args),
        "presets" => cmd_presets(),
        "status" => cmd_status(&args),
        "work" => cmd_work(&args),
        "-h" | "--help" | "help" => {
            print_usage();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown command: {other}");
            print_usage();
            ExitCode::from(1)
        }
    }
}

fn print_usage() {
    eprintln!(
        "\
symdb — per-study / edge catalog and agentic work log

Usage:
  symdb init <dir> [--profile study|node|agent] [--preset NAME ...]
  symdb apply [<dir>] [--profile study|agent]
  symdb presets
  symdb status [<dir>]
  symdb work query open --title T [--parent UUID]
                        [--harness H] [--role R] [--agent A]
                        [--body-sha256 HEX]
  symdb work query finish --id UUID --status done|abandoned
  symdb work attempt start --query UUID --agent A
                           [--kind session|spawn|revise]
                           [--parent-attempt UUID]
  symdb work attempt finish --id UUID --outcome OUTCOME --summary S
  symdb work artifact add --attempt UUID --path REL
                          --role input|output|log [--sha256 HEX]

Study SQLite: <dir>/{DEFAULT_DB_RELATIVE}  (schema v{SCHEMA_VERSION})
Work log:     <dir>/{DEFAULT_AGENT_DB_RELATIVE}  (schema v{AGENTIC_SCHEMA_VERSION})
Prompt text is not stored. Pass --body-sha256.
"
    );
}

fn cmd_presets() -> ExitCode {
    #[cfg(feature = "sqlite")]
    {
        for name in available_presets() {
            println!("{name}");
        }
        ExitCode::SUCCESS
    }
    #[cfg(not(feature = "sqlite"))]
    {
        eprintln!("presets requires --features sqlite");
        ExitCode::from(5)
    }
}

fn cmd_init(args: &[String]) -> ExitCode {
    #[cfg(feature = "sqlite")]
    {
        let mut dir: Option<PathBuf> = None;
        let mut profile = StoreProfile::Study;
        let mut presets: Vec<String> = Vec::new();
        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--profile" => {
                    i += 1;
                    let raw = args.get(i).map(String::as_str).unwrap_or("");
                    match StoreProfile::parse(raw) {
                        Some(p) => profile = p,
                        None => {
                            eprintln!("invalid --profile {raw} (use study, node, or agent)");
                            return ExitCode::from(2);
                        }
                    }
                }
                "--preset" => {
                    i += 1;
                    if let Some(name) = args.get(i) {
                        presets.push(name.clone());
                    }
                }
                flag if flag.starts_with('-') => {
                    eprintln!("unknown flag: {flag}");
                    return ExitCode::from(2);
                }
                other => {
                    if dir.is_some() {
                        eprintln!("unexpected argument: {other}");
                        return ExitCode::from(2);
                    }
                    dir = Some(PathBuf::from(other));
                }
            }
            i += 1;
        }
        let Some(dir) = dir else {
            eprintln!("init requires a directory");
            print_usage();
            return ExitCode::from(2);
        };
        match init(&dir, profile, &presets) {
            Ok(r) => {
                println!(
                    "initialized {} ({})  db={}  schema={}  attributes={}",
                    r.root.display(),
                    r.profile.as_str(),
                    r.db.display(),
                    r.schema.display(),
                    r.attributes
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("init error: {e}");
                ExitCode::from(7)
            }
        }
    }
    #[cfg(not(feature = "sqlite"))]
    {
        let _ = args;
        eprintln!("init requires --features sqlite");
        ExitCode::from(5)
    }
}

fn cmd_apply(args: &[String]) -> ExitCode {
    #[cfg(feature = "sqlite")]
    {
        let mut dir = PathBuf::from(".");
        let mut profile = String::from("study");
        let mut saw_dir = false;
        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--profile" => {
                    i += 1;
                    let raw = args.get(i).map(String::as_str).unwrap_or("");
                    match raw {
                        "study" | "agent" => profile = raw.to_string(),
                        _ => {
                            eprintln!("invalid --profile {raw} (use study or agent)");
                            return ExitCode::from(2);
                        }
                    }
                }
                flag if flag.starts_with('-') => {
                    eprintln!("unknown flag: {flag}");
                    return ExitCode::from(2);
                }
                other => {
                    if saw_dir {
                        eprintln!("unexpected argument: {other}");
                        return ExitCode::from(2);
                    }
                    dir = PathBuf::from(other);
                    saw_dir = true;
                }
            }
            i += 1;
        }
        let applied = if profile == "agent" {
            apply_agent(&dir)
        } else {
            apply(&dir)
        };
        match applied {
            Ok(schema) => {
                println!("applied {}", schema.display());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("apply error: {e}");
                ExitCode::from(7)
            }
        }
    }
    #[cfg(not(feature = "sqlite"))]
    {
        let _ = args;
        eprintln!("apply requires --features sqlite");
        ExitCode::from(5)
    }
}

#[cfg(feature = "sqlite")]
fn print_status(db: &std::path::Path) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use rusqlite::Connection;

    let conn = Connection::open(db)?;
    let version: i32 = conn.query_row("SELECT COALESCE(MAX(version), 0) FROM schema_migrations", [], |r| {
        r.get(0)
    })?;
    let profile: String = conn
        .query_row("SELECT value FROM catalog_meta WHERE key = 'profile'", [], |r| r.get(0))
        .unwrap_or_else(|_| "?".into());
    let subjects: i32 = conn.query_row("SELECT COUNT(*) FROM subjects", [], |r| r.get(0))?;
    let sessions: i32 = conn.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?;
    let files: i32 = conn.query_row("SELECT COUNT(*) FROM file_records", [], |r| r.get(0))?;
    let attrs: i32 = conn.query_row("SELECT COUNT(*) FROM attributes", [], |r| r.get(0))?;
    println!("db          {}", db.display());
    println!("schema      {version}");
    println!("profile     {profile}");
    println!("subjects    {subjects}");
    println!("sessions    {sessions}");
    println!("files       {files}");
    println!("attributes  {attrs}");
    Ok(())
}

#[cfg(feature = "sqlite")]
fn print_agent_status(db: &std::path::Path) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use rusqlite::Connection;

    let conn = Connection::open(db)?;
    let version: i32 = conn.query_row("SELECT COALESCE(MAX(version), 0) FROM schema_migrations", [], |r| {
        r.get(0)
    })?;
    let profile: String = conn
        .query_row("SELECT value FROM catalog_meta WHERE key = 'profile'", [], |r| r.get(0))
        .unwrap_or_else(|_| "?".into());
    let queries: i32 = conn.query_row("SELECT COUNT(*) FROM queries", [], |r| r.get(0))?;
    let attempts: i32 = conn.query_row("SELECT COUNT(*) FROM attempts", [], |r| r.get(0))?;
    let artifacts: i32 = conn.query_row("SELECT COUNT(*) FROM artifacts", [], |r| r.get(0))?;
    println!("db          {}", db.display());
    println!("schema      {version}");
    println!("profile     {profile}");
    println!("queries     {queries}");
    println!("attempts    {attempts}");
    println!("artifacts   {artifacts}");
    Ok(())
}

fn cmd_status(args: &[String]) -> ExitCode {
    #[cfg(feature = "sqlite")]
    {
        let dir = match args.get(2) {
            None => PathBuf::from("."),
            Some(s) if !s.starts_with('-') && args.len() == 3 => PathBuf::from(s),
            Some(s) if s.starts_with('-') => {
                eprintln!("unknown flag: {s}");
                return ExitCode::from(2);
            }
            Some(_) => {
                eprintln!("unexpected argument: {}", args[3]);
                return ExitCode::from(2);
            }
        };
        let study = dir.join(DEFAULT_DB_RELATIVE);
        let agent = dir.join(DEFAULT_AGENT_DB_RELATIVE);
        if !study.exists() && !agent.exists() {
            eprintln!(
                "no catalog at {} and no work log at {} — run: symdb init {}",
                study.display(),
                agent.display(),
                dir.display()
            );
            return ExitCode::from(8);
        }
        if study.exists()
            && let Err(e) = print_status(&study)
        {
            eprintln!("status error: {e}");
            return ExitCode::from(8);
        }
        if agent.exists() {
            if study.exists() {
                println!();
            }
            if let Err(e) = print_agent_status(&agent) {
                eprintln!("status error: {e}");
                return ExitCode::from(8);
            }
        }
        ExitCode::SUCCESS
    }
    #[cfg(not(feature = "sqlite"))]
    {
        let _ = args;
        eprintln!("status requires --features sqlite");
        ExitCode::from(5)
    }
}

fn cmd_work(args: &[String]) -> ExitCode {
    #[cfg(feature = "sqlite")]
    {
        if args.len() < 4 {
            eprintln!("work requires a noun and a verb");
            print_usage();
            return ExitCode::from(2);
        }
        let flags = match parse_work_flags(args) {
            Ok(flags) => flags,
            Err(code) => return code,
        };
        match (args[2].as_str(), args[3].as_str()) {
            ("query", "open") => work_query_open(&flags),
            ("query", "finish") => work_query_finish(&flags),
            ("attempt", "start") => work_attempt_start(&flags),
            ("attempt", "finish") => work_attempt_finish(&flags),
            ("artifact", "add") => work_artifact_add(&flags),
            _ => {
                eprintln!("unknown work command: {} {}", args[2], args[3]);
                print_usage();
                ExitCode::from(2)
            }
        }
    }
    #[cfg(not(feature = "sqlite"))]
    {
        let _ = args;
        eprintln!("work requires --features sqlite");
        ExitCode::from(5)
    }
}

#[cfg(feature = "sqlite")]
fn parse_work_flags(args: &[String]) -> std::result::Result<BTreeMap<String, String>, ExitCode> {
    let mut map = BTreeMap::new();
    let mut i = 4;
    while i < args.len() {
        let flag = args[i].as_str();
        let Some(key) = flag.strip_prefix("--") else {
            eprintln!("unexpected argument: {flag}");
            return Err(ExitCode::from(2));
        };
        if key.is_empty() {
            eprintln!("unknown flag: {flag}");
            return Err(ExitCode::from(2));
        }
        if key == "body" || key == "body-file" {
            eprintln!("v1 stores a hash, not the prompt");
            return Err(ExitCode::from(2));
        }
        i += 1;
        let Some(value) = args.get(i) else {
            eprintln!("missing value for --{key}");
            return Err(ExitCode::from(2));
        };
        map.insert(key.to_string(), value.clone());
        i += 1;
    }
    Ok(map)
}

#[cfg(feature = "sqlite")]
fn allow_keys(flags: &BTreeMap<String, String>, allowed: &[&str]) -> std::result::Result<(), ExitCode> {
    for key in flags.keys() {
        if !allowed.contains(&key.as_str()) {
            eprintln!("unknown flag: --{key}");
            return Err(ExitCode::from(2));
        }
    }
    Ok(())
}

#[cfg(feature = "sqlite")]
fn required<'a>(flags: &'a BTreeMap<String, String>, key: &str) -> std::result::Result<&'a str, ExitCode> {
    match flags.get(key).map(String::as_str).filter(|value| !value.is_empty()) {
        Some(value) => Ok(value),
        None => {
            eprintln!("missing --{key}");
            Err(ExitCode::from(2))
        }
    }
}

#[cfg(feature = "sqlite")]
fn optional<'a>(flags: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    flags.get(key).map(String::as_str).filter(|value| !value.is_empty())
}

#[cfg(feature = "sqlite")]
fn agent_conn() -> std::result::Result<rusqlite::Connection, ExitCode> {
    let db = PathBuf::from(DEFAULT_AGENT_DB_RELATIVE);
    if !db.is_file() {
        eprintln!("no work log at {} — run: symdb init --profile agent", db.display());
        return Err(ExitCode::from(7));
    }
    match open_agent(&db) {
        Ok(conn) => Ok(conn),
        Err(e) => {
            eprintln!("work error: {e}");
            Err(ExitCode::from(7))
        }
    }
}

#[cfg(feature = "sqlite")]
fn work_query_open(flags: &BTreeMap<String, String>) -> ExitCode {
    if let Err(code) = allow_keys(flags, &["title", "parent", "harness", "role", "agent", "body-sha256"]) {
        return code;
    }
    let title = match required(flags, "title") {
        Ok(value) => value,
        Err(code) => return code,
    };
    let conn = match agent_conn() {
        Ok(conn) => conn,
        Err(code) => return code,
    };
    match open_query(
        &conn,
        title,
        optional(flags, "parent"),
        optional(flags, "harness"),
        optional(flags, "role"),
        optional(flags, "agent"),
        optional(flags, "body-sha256"),
    ) {
        Ok(id) => {
            println!("{id}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("work error: {e}");
            ExitCode::from(7)
        }
    }
}

#[cfg(feature = "sqlite")]
fn work_query_finish(flags: &BTreeMap<String, String>) -> ExitCode {
    if let Err(code) = allow_keys(flags, &["id", "status"]) {
        return code;
    }
    let id = match required(flags, "id") {
        Ok(value) => value,
        Err(code) => return code,
    };
    let status = match required(flags, "status") {
        Ok(value) => value,
        Err(code) => return code,
    };
    let conn = match agent_conn() {
        Ok(conn) => conn,
        Err(code) => return code,
    };
    match finish_query(&conn, id, status) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("work error: {e}");
            ExitCode::from(7)
        }
    }
}

#[cfg(feature = "sqlite")]
fn work_attempt_start(flags: &BTreeMap<String, String>) -> ExitCode {
    if let Err(code) = allow_keys(flags, &["query", "agent", "kind", "parent-attempt"]) {
        return code;
    }
    let query = match required(flags, "query") {
        Ok(value) => value,
        Err(code) => return code,
    };
    let agent = match required(flags, "agent") {
        Ok(value) => value,
        Err(code) => return code,
    };
    let conn = match agent_conn() {
        Ok(conn) => conn,
        Err(code) => return code,
    };
    match start_attempt(
        &conn,
        query,
        agent,
        optional(flags, "kind"),
        optional(flags, "parent-attempt"),
    ) {
        Ok(id) => {
            println!("{id}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("work error: {e}");
            ExitCode::from(7)
        }
    }
}

#[cfg(feature = "sqlite")]
fn work_attempt_finish(flags: &BTreeMap<String, String>) -> ExitCode {
    if let Err(code) = allow_keys(flags, &["id", "outcome", "summary"]) {
        return code;
    }
    let id = match required(flags, "id") {
        Ok(value) => value,
        Err(code) => return code,
    };
    let outcome = match required(flags, "outcome") {
        Ok(value) => value,
        Err(code) => return code,
    };
    let summary = match required(flags, "summary") {
        Ok(value) => value,
        Err(code) => return code,
    };
    let conn = match agent_conn() {
        Ok(conn) => conn,
        Err(code) => return code,
    };
    match finish_attempt(&conn, id, outcome, summary) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("work error: {e}");
            ExitCode::from(7)
        }
    }
}

#[cfg(feature = "sqlite")]
fn work_artifact_add(flags: &BTreeMap<String, String>) -> ExitCode {
    if let Err(code) = allow_keys(flags, &["attempt", "path", "role", "sha256"]) {
        return code;
    }
    let attempt = match required(flags, "attempt") {
        Ok(value) => value,
        Err(code) => return code,
    };
    let path = match required(flags, "path") {
        Ok(value) => value,
        Err(code) => return code,
    };
    let role = match required(flags, "role") {
        Ok(value) => value,
        Err(code) => return code,
    };
    let conn = match agent_conn() {
        Ok(conn) => conn,
        Err(code) => return code,
    };
    match add_artifact(&conn, attempt, path, role, optional(flags, "sha256")) {
        Ok(id) => {
            println!("{id}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("work error: {e}");
            ExitCode::from(7)
        }
    }
}
