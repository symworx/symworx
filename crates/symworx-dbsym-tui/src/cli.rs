// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Argv for `symdb-view`. No clap: same style as `symdb`.

use std::path::{
    Path,
    PathBuf,
};

use symworx_dbsym::DEFAULT_DB_RELATIVE;

use crate::error::{
    Error,
    Result,
};

/// What the process should do.
#[derive(Debug, PartialEq, Eq)]
pub enum Cli {
    /// Print usage and exit 0.
    Help,
    /// Open one catalog.
    Open(Target),
}

/// SQLite file or Postgres URL.
#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    /// Filesystem path.
    Sqlite(PathBuf),
    /// `postgres://` or `postgresql://` URL, or a libpq key/value string passed via `--url`.
    Postgres(String),
}

/// Parse arguments after the program name. `cwd` resolves relative SQLite paths.
pub fn parse_args(args: &[String], cwd: &Path) -> Result<Cli> {
    if args.is_empty() {
        return Ok(Cli::Open(Target::Sqlite(cwd.join(DEFAULT_DB_RELATIVE))));
    }
    if args.len() == 1 && matches!(args[0].as_str(), "-h" | "--help" | "help") {
        return Ok(Cli::Help);
    }

    let mut url: Option<String> = None;
    let mut positional: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--url" {
            index += 1;
            let Some(value) = args.get(index) else {
                return Err(Error::InvalidParameter("--url needs a connection URL".to_string()));
            };
            if url.is_some() {
                return Err(Error::InvalidParameter("only one --url is allowed".to_string()));
            }
            url = Some(value.clone());
        } else if let Some(value) = arg.strip_prefix("--url=") {
            if value.is_empty() {
                return Err(Error::InvalidParameter("--url needs a connection URL".to_string()));
            }
            if url.is_some() {
                return Err(Error::InvalidParameter("only one --url is allowed".to_string()));
            }
            url = Some(value.to_string());
        } else if arg.starts_with('-') {
            return Err(Error::InvalidParameter(format!("unknown argument: {arg}")));
        } else if positional.is_some() {
            return Err(Error::InvalidParameter("too many arguments".to_string()));
        } else {
            positional = Some(arg.clone());
        }
        index += 1;
    }

    match (url, positional) {
        (Some(_), Some(_)) => Err(Error::InvalidParameter(
            "pass either a SQLite path or --url, not both".to_string(),
        )),
        (Some(url), None) => Ok(Cli::Open(Target::Postgres(url))),
        (None, Some(value)) if is_postgres_url(&value) => Ok(Cli::Open(Target::Postgres(value))),
        (None, Some(value)) => Ok(Cli::Open(Target::Sqlite(resolve(cwd, &value)))),
        (None, None) => Ok(Cli::Open(Target::Sqlite(cwd.join(DEFAULT_DB_RELATIVE)))),
    }
}

fn is_postgres_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("postgres://") || lower.starts_with("postgresql://")
}

fn resolve(cwd: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() { path } else { cwd.join(path) }
}

/// Usage printed to stderr for errors and stdout for `--help`.
pub fn usage() -> String {
    format!(
        "\
symdb-view — read-only browser for a dbSym catalog

Usage:
  symdb-view                         open ./{}
  symdb-view <file.sqlite>
  symdb-view --url <postgres-url>
  symdb-view postgres://user@host:5432/db

Password: in the URL, or PGPASSWORD. TLS (sslmode=require) is not built in.
The session is read-only. This is not symview and not the LoadSym catalog.
",
        DEFAULT_DB_RELATIVE
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use symworx_dbsym::DEFAULT_DB_RELATIVE;

    use super::{
        Cli,
        Target,
        parse_args,
    };

    fn parse(args: &[&str]) -> Cli {
        let owned: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
        parse_args(&owned, Path::new("/study")).unwrap()
    }

    #[test]
    fn default_is_the_dbsym_file_under_cwd() {
        match parse(&[]) {
            Cli::Open(Target::Sqlite(path)) => {
                assert_eq!(path, Path::new("/study").join(DEFAULT_DB_RELATIVE));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn positional_sqlite_and_postgres_url() {
        match parse(&["catalog.sqlite"]) {
            Cli::Open(Target::Sqlite(path)) => assert_eq!(path, Path::new("/study/catalog.sqlite")),
            other => panic!("unexpected {other:?}"),
        }
        match parse(&["/data/dbsym.sqlite"]) {
            Cli::Open(Target::Sqlite(path)) => assert_eq!(path, Path::new("/data/dbsym.sqlite")),
            other => panic!("unexpected {other:?}"),
        }
        match parse(&["postgres://alice@localhost:5432/study"]) {
            Cli::Open(Target::Postgres(url)) => {
                assert!(url.starts_with("postgres://"));
            }
            other => panic!("unexpected {other:?}"),
        }
        match parse(&["--url", "postgresql://localhost/db"]) {
            Cli::Open(Target::Postgres(url)) => assert_eq!(url, "postgresql://localhost/db"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn help_and_rejects_mixed_targets() {
        assert_eq!(parse(&["--help"]), Cli::Help);
        let owned = vec![
            "--url".to_string(),
            "postgres://localhost/db".to_string(),
            "file.sqlite".to_string(),
        ];
        let err = parse_args(&owned, Path::new("/study")).unwrap_err();
        assert!(err.to_string().contains("not both"));
    }
}
