// symload — headless CLI for activity stats, email fetch, and personal SQLite catalog.
//
// Personal data (DB, FIT archive, credentials) lives under $VELOFIT_HOME (default ~/velofit).
// This binary never embeds personal emails, bucket names, or sample athlete rows.

use std::{
    env,
    fs,
    path::{
        Path,
        PathBuf,
    },
};

#[cfg(feature = "email")]
use symworx_io::email;
use symworx_io::{
    default_velofit_inbox,
    default_velofit_raw,
    default_velofit_root,
    discover_activity_files,
    load_activity,
};
use symworx_loadsym::load::compute_ride_metrics;
#[cfg(feature = "db")]
use symworx_loadsym_db;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let cmd = &args[1];
    match cmd.as_str() {
        "db" | "schema" => {
            if let Err(e) = handle_db_command(&args) {
                eprintln!("db error: {}", e);
                std::process::exit(7);
            }
        }
        "ingest" | "reprocess" => {
            #[cfg(feature = "sqlite")]
            {
                let force = cmd == "reprocess" || args.iter().any(|a| a == "--force" || a == "-F");
                if let Err(e) = handle_ingest(&args, force) {
                    eprintln!("ingest error: {}", e);
                    std::process::exit(8);
                }
            }
            #[cfg(not(feature = "sqlite"))]
            {
                eprintln!("ingest requires --features sqlite (includes fit + db)");
                std::process::exit(5);
            }
        }
        "ftp" => {
            #[cfg(feature = "sqlite")]
            {
                if let Err(e) = handle_ftp(&args) {
                    eprintln!("ftp error: {}", e);
                    std::process::exit(9);
                }
            }
            #[cfg(not(feature = "sqlite"))]
            {
                eprintln!("ftp commands require --features sqlite");
                std::process::exit(5);
            }
        }
        "email" | "fetch" => {
            #[cfg(feature = "email")]
            {
                if let Err(e) = handle_email_fetch(&args) {
                    eprintln!("email error: {}", e);
                    std::process::exit(4);
                }
            }
            #[cfg(not(feature = "email"))]
            {
                eprintln!("Email support requires building with --features email");
                std::process::exit(5);
            }
        }
        "inbox" => {
            if args.get(2).map(|s| s.as_str()) == Some("promote") {
                if let Err(e) = handle_inbox_promote(&args) {
                    eprintln!("inbox promote error: {}", e);
                    std::process::exit(6);
                }
            } else {
                eprintln!("Usage: symload inbox promote [--from DIR] [--to DIR]");
                std::process::exit(2);
            }
        }
        "stats" | "stat" => {
            if args.len() < 3 {
                eprintln!("Usage: symload stats <file.fit | dir>");
                std::process::exit(2);
            }
            let target = &args[2];
            let ftp = parse_ftp(&args);
            let json_only = args.iter().any(|a| a == "--json" || a == "-j");

            let paths = if Path::new(target).is_dir() {
                find_fit_files(target)
            } else {
                vec![PathBuf::from(target)]
            };

            if paths.is_empty() {
                eprintln!("No .fit files found at {}", target);
                std::process::exit(3);
            }

            for p in paths {
                if let Err(e) = process_one(&p, ftp, json_only) {
                    eprintln!("{}: {}", p.display(), e);
                }
            }
        }
        "-h" | "--help" | "help" => print_usage(),
        _ => {
            eprintln!("Unknown command: {}", cmd);
            print_usage();
            std::process::exit(1);
        }
    }
}

fn handle_db_command(args: &[String]) -> Result<(), String> {
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("help");
    match sub {
        "print-schema" | "schema" => {
            #[cfg(feature = "db")]
            {
                let dialect = if args.iter().any(|a| a == "--postgres" || a == "postgres") {
                    "postgres"
                } else if args.iter().any(|a| a == "--sqlite" || a == "sqlite") {
                    "sqlite"
                } else {
                    "sqlite"
                };
                print!("{}", symworx_loadsym_db::get_schema(dialect));
                Ok(())
            }
            #[cfg(not(feature = "db"))]
            {
                Err("print-schema requires --features db".into())
            }
        }
        "init" => {
            #[cfg(feature = "sqlite")]
            {
                let db = parse_db_path(args);
                symworx_loadsym::catalog::init_catalog(&db)?;
                println!("Initialized SQLite catalog at {}", db.display());
                println!("(personal data — keep this file outside the source tree / git)");
                Ok(())
            }
            #[cfg(not(feature = "sqlite"))]
            {
                let _ = args;
                Err("db init requires --features sqlite".into())
            }
        }
        "status" => {
            #[cfg(feature = "sqlite")]
            {
                let db = parse_db_path(args);
                let conn = symworx_loadsym::catalog::open_catalog(&db)?;
                let n = symworx_loadsym::catalog::count_activities(&conn)?;
                let ftp_n: i64 = conn
                    .query_row("SELECT COUNT(*) FROM ftp_history", [], |r| r.get(0))
                    .unwrap_or(0);
                println!("db: {}", db.display());
                println!("activities: {}", n);
                println!("ftp_history rows: {}", ftp_n);
                Ok(())
            }
            #[cfg(not(feature = "sqlite"))]
            {
                let _ = args;
                Err("db status requires --features sqlite".into())
            }
        }
        _ => {
            eprintln!("symload db <print-schema|init|status> [--db PATH]");
            Err("unknown db subcommand".into())
        }
    }
}

#[cfg(feature = "sqlite")]
fn handle_ftp(args: &[String]) -> Result<(), String> {
    use symworx_loadsym::catalog::{
        list_ftp_history,
        open_catalog,
        set_ftp_history,
    };

    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("list");
    let db = parse_db_path(args);
    if !db.exists() {
        symworx_loadsym::catalog::init_catalog(&db)?;
    }
    let conn = open_catalog(&db)?;

    match sub {
        "list" | "ls" => {
            let rows = list_ftp_history(&conn)?;
            if rows.is_empty() {
                println!("(no ftp_history rows — use: symload ftp set --date YYYY-MM-DD --ftp N)");
            }
            for (id, from, to, ftp, sport, source) in rows {
                println!(
                    "id={}  {} → {}  ftp={:.0}W  sport={}  source={}",
                    id,
                    from,
                    to.as_deref().unwrap_or("…"),
                    ftp,
                    sport,
                    source.as_deref().unwrap_or("-")
                );
            }
            Ok(())
        }
        "set" => {
            let date = parse_flag_value(args, "--date")
                .or_else(|| parse_flag_value(args, "--from"))
                .ok_or_else(|| "ftp set requires --date YYYY-MM-DD".to_string())?;
            let ftp: f64 = parse_flag_value(args, "--ftp")
                .or_else(|| parse_flag_value(args, "-f"))
                .ok_or_else(|| "ftp set requires --ftp WATTS".to_string())?
                .parse()
                .map_err(|_| "invalid --ftp".to_string())?;
            let sport = parse_flag_value(args, "--sport").unwrap_or_else(|| "cycling".into());
            let source = parse_flag_value(args, "--source");
            let notes = parse_flag_value(args, "--notes");
            let until = parse_flag_value(args, "--until");
            let id = set_ftp_history(
                &conn,
                &date,
                ftp,
                &sport,
                source.as_deref(),
                notes.as_deref(),
                until.as_deref(),
            )?;
            println!(
                "ftp_history id={}  effective_from={}  ftp={:.0}W  sport={}",
                id, date, ftp, sport
            );
            println!("Re-score rides: symload reprocess --ftp {:.0}", ftp);
            Ok(())
        }
        _ => Err("Usage: symload ftp list | ftp set --date YYYY-MM-DD --ftp N [--sport cycling] [--source manual] [--until YYYY-MM-DD]".into()),
    }
}

fn parse_flag_value(args: &[String], flag: &str) -> Option<String> {
    for (i, a) in args.iter().enumerate() {
        if a == flag && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    None
}

/// First non-flag positional path for `email fetch` (skips --query/-q values).
#[cfg(feature = "email")]
fn parse_email_target_dir(args: &[String]) -> PathBuf {
    // args[0]=bin, args[1]=email|fetch; if args[1]==email then args[2] may be "fetch"
    let start = if args.get(1).map(|s| s.as_str()) == Some("email") {
        if args.get(2).map(|s| s.as_str()) == Some("fetch") {
            3
        } else {
            2
        }
    } else {
        2
    };
    let mut i = start;
    while i < args.len() {
        let a = &args[i];
        if a == "--query" || a == "-q" {
            i += 2;
            continue;
        }
        if a.starts_with('-') {
            i += 1;
            continue;
        }
        return PathBuf::from(a);
    }
    default_velofit_inbox()
}

/// First positional path after the command (skips known flags/values).
fn parse_ingest_target(args: &[String]) -> PathBuf {
    let mut i = 2usize;
    while i < args.len() {
        let a = &args[i];
        if a == "--force" || a == "-F" {
            i += 1;
            continue;
        }
        if matches!(
            a.as_str(),
            "--ftp"
                | "-f"
                | "--db"
                | "-d"
                | "--date"
                | "--from"
                | "--sport"
                | "--source"
                | "--notes"
                | "--until"
        ) {
            i += 2;
            continue;
        }
        if a.starts_with('-') {
            i += 1;
            continue;
        }
        return PathBuf::from(a);
    }
    default_velofit_raw()
}

#[cfg(feature = "sqlite")]
fn handle_ingest(args: &[String], force: bool) -> Result<(), String> {
    use symworx_loadsym::catalog::{
        IngestOutcome,
        ingest_one,
        open_catalog,
        recompute_load_metrics,
    };

    let db = parse_db_path(args);
    let ftp = parse_ftp(args);
    let target = parse_ingest_target(args);

    if !db.exists() {
        symworx_loadsym::catalog::init_catalog(&db)?;
        println!("Created catalog at {}", db.display());
    }
    let conn = open_catalog(&db)?;
    let archive_root = default_velofit_root();

    let paths = if target.is_dir() {
        find_fit_files(target.to_str().unwrap_or("."))
    } else {
        vec![target]
    };

    if paths.is_empty() {
        return Err("No .fit files found for ingest/reprocess".into());
    }

    if force {
        println!(
            "reprocess mode: re-scoring with ftp_history (fallback FTP={:.0})",
            ftp
        );
    }

    let mut inserted = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for p in &paths {
        match ingest_one(&conn, p, ftp, Some(&archive_root), force) {
            IngestOutcome::Inserted {
                source_key,
                tss,
                ftp_w,
                ftp_origin,
            } => {
                inserted += 1;
                println!(
                    "+ {}  TSS={:.1}  FTP={:.0}W ({})",
                    source_key, tss, ftp_w, ftp_origin
                );
            }
            IngestOutcome::Skipped { source_key, reason } => {
                skipped += 1;
                if force || env::var("SYMLOAD_INGEST_VERBOSE").is_ok() {
                    println!("= {}  ({})", source_key, reason);
                }
            }
            IngestOutcome::Failed { path, error } => {
                failed += 1;
                eprintln!("! {}: {}", path, error);
            }
        }
    }

    // Rebuild daily rollups from activities (drops stale mtime-pile days after re-dating).
    let days_n = symworx_loadsym::catalog::recompute_all_daily_loads(&conn)?;
    let metrics_n = recompute_load_metrics(&conn)?;
    println!(
        "ingest done: inserted/updated={} skipped={} failed={}  daily_days={}  load_metrics_rows={}",
        inserted, skipped, failed, days_n, metrics_n
    );
    println!("db: {}", db.display());
    Ok(())
}

fn parse_db_path(args: &[String]) -> PathBuf {
    for (i, a) in args.iter().enumerate() {
        if (a == "--db" || a == "-d") && i + 1 < args.len() {
            return PathBuf::from(&args[i + 1]);
        }
    }
    #[cfg(feature = "sqlite")]
    {
        return symworx_loadsym::catalog::default_catalog_path();
    }
    #[cfg(not(feature = "sqlite"))]
    {
        default_velofit_root().join("db").join("loadsym.sqlite")
    }
}

#[cfg(feature = "email")]
fn handle_email_fetch(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let user = env::var("SYMLOAD_USER").map_err(|_| {
        "set SYMLOAD_USER to your IMAP username (do not commit credentials)".to_string()
    })?;
    let pass = env::var("SYMLOAD_APP_PASSWORD")
        .map_err(|_| "set SYMLOAD_APP_PASSWORD (app password; do not commit)".to_string())?;

    // Optional IMAP SEARCH query. Default matches SRM PC8 export subjects.
    // Examples: --query "SUBJECT SRM UNSEEN"
    //           --query "OR SUBJECT SRM SUBJECT Polar"
    let query = parse_flag_value(args, "--query")
        .or_else(|| parse_flag_value(args, "-q"))
        .unwrap_or_else(|| "SUBJECT SRM".into());

    // Target directory: first non-flag positional after the command, else inbox.
    // Usage: symload email fetch [target_dir] [--query "..."]
    let target = parse_email_target_dir(args);

    let saved = if query == "SUBJECT SRM" {
        email::fetch_srm_fit_attachments(&user, &pass, &target)?
    } else {
        email::fetch_fit_attachments(&user, &pass, &target, &query)?
    };

    println!(
        "Fetched {} new .fit file(s) to {}  (query: {})",
        saved.len(),
        target.display(),
        query
    );
    for p in &saved {
        println!("  {}", p.display());
    }
    if saved.is_empty() {
        println!("(none new — already present or no matching attachments)");
    }
    Ok(())
}

fn handle_inbox_promote(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut from = default_velofit_inbox();
    let mut to = default_velofit_raw();

    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--from" if i + 1 < args.len() => {
                from = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--to" if i + 1 < args.len() => {
                to = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            other => {
                eprintln!("Unknown arg for inbox promote: {}", other);
                std::process::exit(2);
            }
        }
    }

    fs::create_dir_all(&to)?;
    if !from.exists() {
        eprintln!("inbox dir does not exist: {}", from.display());
        std::process::exit(3);
    }

    let entries = discover_activity_files(&[from.clone()], false);
    let mut moved = 0usize;
    let mut skipped = 0usize;

    for e in entries {
        let p = &e.path;
        let Some(ext) = p.extension().and_then(|x| x.to_str()) else {
            continue;
        };
        if !ext.eq_ignore_ascii_case("fit") {
            continue;
        }
        let Some(name) = p.file_name() else {
            continue;
        };
        let dest = to.join(name);
        if dest.exists() {
            let src_len = fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            let dst_len = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
            if src_len == dst_len {
                let _ = fs::remove_file(p);
                skipped += 1;
                continue;
            }
            eprintln!(
                "skip (dest exists, different size): {} vs {}",
                p.display(),
                dest.display()
            );
            skipped += 1;
            continue;
        }
        fs::rename(p, &dest).or_else(|_| fs::copy(p, &dest).and_then(|_| fs::remove_file(p)))?;
        println!("promoted {}", dest.display());
        moved += 1;
    }

    println!(
        "inbox promote: moved={} skipped={}  (from={} → to={})",
        moved,
        skipped,
        from.display(),
        to.display()
    );
    Ok(())
}

fn parse_ftp(args: &[String]) -> f64 {
    for (i, a) in args.iter().enumerate() {
        if (a == "--ftp" || a == "-f") && i + 1 < args.len() {
            if let Ok(v) = args[i + 1].parse::<f64>() {
                return v.max(50.0);
            }
        }
    }
    280.0
}

fn find_fit_files(dir: &str) -> Vec<PathBuf> {
    discover_activity_files(&[PathBuf::from(dir)], false)
        .into_iter()
        .map(|e| e.path)
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("fit"))
                .unwrap_or(false)
        })
        .collect()
}

fn process_one(path: &Path, ftp: f64, json_only: bool) -> Result<(), Box<dyn std::error::Error>> {
    let act = load_activity(&path.to_string_lossy())?;

    let n = act.len();
    let dur = act.duration_s();
    let p = act.power_series();
    let avg = if n > 0 {
        p.iter().sum::<f64>() / n as f64
    } else {
        0.0
    };
    let maxp = p.iter().copied().fold(0.0, f64::max);

    let m = compute_ride_metrics(&act.times_s, &p, ftp);

    if json_only {
        println!(
            "{{\"file\":\"{}\",\"duration_s\":{:.1},\"avg_w\":{:.1},\"max_w\":{:.0},\"np\":{:.0},\"if\":{:.2},\"tss\":{:.1},\"ftp\":{:.0}}}",
            path.display(),
            dur,
            avg,
            maxp,
            m.np,
            m.if_,
            m.tss,
            ftp
        );
        return Ok(());
    }

    println!("\n=== {} ===", path.display());
    println!("duration: {:.1} min ({} samples)", dur / 60.0, n);
    if let Some(mfr) = &act.manufacturer {
        println!("device: {} {}", mfr, act.product.as_deref().unwrap_or(""));
    }
    if let Some(s) = &act.sport {
        println!("sport: {}", s);
    }
    println!(
        "power: avg={:.1}W  max={:.0}W  has_power={}",
        avg,
        maxp,
        act.has_power()
    );
    println!(
        "FTP={}W → NP={:.0}W  IF={:.2}  TSS={:.1}  work={:.0}kJ",
        ftp, m.np, m.if_, m.tss, m.total_work_kj
    );

    Ok(())
}

fn print_usage() {
    eprintln!(
        r#"symload — activity stats + personal catalog (symworx-loadsym)

Commands:
  symload stats <file.fit | dir> [--ftp 280] [--json]
  symload db print-schema [--sqlite|--postgres]
  symload db init [--db PATH]          (needs --features sqlite)
  symload db status [--db PATH]
  symload ingest [path|dir] [--db PATH] [--ftp 280] [--force]
      FTP for each ride comes from ftp_history when set; --ftp is fallback only.
  symload reprocess [path|dir] [--ftp 280]   same as ingest --force (re-score loads)
  symload ftp list
  symload ftp set --date YYYY-MM-DD --ftp N [--sport cycling] [--source manual] [--until DATE]
  symload email fetch [target_dir] [--query "SUBJECT SRM"]
      default dir: $VELOFIT_HOME/inbox; default query matches SRM exports
  symload inbox promote [--from DIR] [--to DIR]

Environment (never commit secrets):
  VELOFIT_HOME          archive root (default: ~/velofit)
  SYMLOAD_DB            SQLite path override
  SYMLOAD_USER          IMAP username for email fetch
  SYMLOAD_APP_PASSWORD  IMAP app password
  SYMLOAD_INGEST_VERBOSE  set to log skipped files

Privacy: the catalog and FIT archive live under VELOFIT_HOME only — not in the SymWorx repo.
"#
    );
}
