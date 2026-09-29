// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! `symdb-view` — read-only table browser for a dbSym SQLite file or Postgres URL.

use std::{
    env,
    process::ExitCode,
};

use symworx_dbsym_tui::{
    Cli,
    browse,
    parse_args,
    usage,
};

fn main() -> ExitCode {
    let mut args = env::args();
    let _program = args.next();
    let args: Vec<String> = args.collect();
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(err) => {
            eprintln!("symdb-view: {err}");
            return ExitCode::from(1);
        }
    };
    let cli = match parse_args(&args, &cwd) {
        Ok(cli) => cli,
        Err(err) => {
            eprintln!("symdb-view: {err}");
            eprint!("{}", usage());
            return ExitCode::from(1);
        }
    };
    match cli {
        Cli::Help => {
            print!("{}", usage());
            ExitCode::SUCCESS
        }
        Cli::Open(target) => match browse(&target) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("symdb-view: {err}");
                ExitCode::from(1)
            }
        },
    }
}
