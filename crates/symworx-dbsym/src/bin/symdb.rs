// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! `symdb` — init a per-study (or edge) SQLite catalog.
//!
//! Data files live under `<dir>/.dbsym/` (never in the SymWorx tree).

use std::{
    env,
    path::PathBuf,
    process::ExitCode,
};

use symworx_dbsym::{
    DEFAULT_DB_RELATIVE,
    SCHEMA_VERSION,
};
#[cfg(feature = "sqlite")]
use symworx_dbsym::{
    StoreProfile,
    init::{
        apply,
        init,
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
symdb — per-study / edge research catalog

Usage:
  symdb init <dir> [--profile study|node] [--preset NAME ...]
  symdb apply [<dir>]     re-apply <dir>/.dbsym/schema.sqlite.sql (study-owned)
  symdb presets
  symdb status [<dir>]

SQLite file: <dir>/{DEFAULT_DB_RELATIVE}  (schema v{SCHEMA_VERSION})
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
                            eprintln!("invalid --profile {raw} (use study or node)");
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
        let dir = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        match apply(&dir) {
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

fn cmd_status(args: &[String]) -> ExitCode {
    #[cfg(feature = "sqlite")]
    {
        let dir = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        let db = dir.join(DEFAULT_DB_RELATIVE);
        if !db.exists() {
            eprintln!("no catalog at {} — run: symdb init {}", db.display(), dir.display());
            return ExitCode::from(8);
        }
        match print_status(&db) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("status error: {e}");
                ExitCode::from(8)
            }
        }
    }
    #[cfg(not(feature = "sqlite"))]
    {
        let _ = args;
        eprintln!("status requires --features sqlite");
        ExitCode::from(5)
    }
}
