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
                let last = symworx_loadsym::catalog::meta_get(
                    &conn,
                    symworx_loadsym::catalog::META_LAST_INGEST_AT,
                )?;
                println!("db: {}", db.display());
                println!("activities: {}", n);
                println!("ftp_history rows: {}", ftp_n);
                match last {
                    Some(t) => println!("last_ingest_at: {}", t),
                    None => println!("last_ingest_at: (none — next ingest scans all files once)"),
                }
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
        if a == "--force"
            || a == "-F"
            || a == "--all"
            || a == "-a"
            || a == "--since-all"
            || a == "--no-watermark"
        {
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
                | "--since"
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

/// `ingest --all` / `-a`: ignore last_ingest_at watermark (still skip known hashes unless --force).
fn ingest_scan_all(args: &[String]) -> bool {
    args.iter()
        .any(|a| a == "--all" || a == "-a" || a == "--since-all" || a == "--no-watermark")
}

#[cfg(feature = "sqlite")]
fn handle_ingest(args: &[String], force: bool) -> Result<(), String> {
    use symworx_loadsym::catalog::{
        IngestOutcome,
        META_LAST_INGEST_AT,
        filter_paths_since_mtime,
        ingest_one,
        meta_get,
        meta_set,
        open_catalog,
        parse_meta_timestamp,
        recompute_load_metrics,
    };

    let db = parse_db_path(args);
    let ftp = parse_ftp(args);
    let target = parse_ingest_target(args);
    // --force reprocess implies full scan; --all also full scan without re-score.
    let scan_all = force || ingest_scan_all(args);

    if !db.exists() {
        symworx_loadsym::catalog::init_catalog(&db)?;
        println!("Created catalog at {}", db.display());
    }
    let conn = open_catalog(&db)?;
    let archive_root = default_velofit_root();

    let all_paths = if target.is_dir() {
        find_fit_files(target.to_str().unwrap_or("."))
    } else {
        vec![target.clone()]
    };

    if all_paths.is_empty() {
        return Err("No .fit files found for ingest/reprocess".into());
    }

    let watermark = meta_get(&conn, META_LAST_INGEST_AT)?;
    let since_unix = if scan_all {
        None
    } else {
        watermark.as_deref().and_then(parse_meta_timestamp)
    };

    let total_on_disk = all_paths.len();
    let paths = filter_paths_since_mtime(all_paths, since_unix);
    let filtered_out = total_on_disk.saturating_sub(paths.len());

    if force {
        println!(
            "reprocess mode: re-scoring ALL candidates with ftp_history (fallback FTP={:.0})",
            ftp
        );
    } else if scan_all {
        println!(
            "full scan (--all): checking {} file(s); known file_hash still skipped",
            paths.len()
        );
    } else if let Some(ref wm) = watermark {
        println!(
            "incremental: last_ingest_at={}  candidates={}  skipped_by_mtime={} (use --all to recheck everything)",
            wm,
            paths.len(),
            filtered_out
        );
    } else {
        println!(
            "first watermark run: scanning all {} file(s); will set last_ingest_at when done",
            paths.len()
        );
    }

    if paths.is_empty() {
        println!(
            "ingest done: nothing newer than watermark ({} on disk; use --all to recheck)",
            total_on_disk
        );
        println!("db: {}", db.display());
        return Ok(());
    }

    let mut inserted = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let n_paths = paths.len();

    for (i, p) in paths.iter().enumerate() {
        let n = i + 1;
        if n == 1 || n == n_paths || n % 100 == 0 {
            eprintln!("ingest progress {n}/{n_paths}  (+{inserted} ={skipped} !{failed})");
        }
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

    // Advance watermark only on non-force incremental/full success (failures still advance;
    // failed files can be retried with --all). Use UTC-ish sqlite now.
    if !force {
        meta_set(&conn, META_LAST_INGEST_AT, &sqlite_utcnow()?)?;
    }

    let wm_now = meta_get(&conn, META_LAST_INGEST_AT)?;
    println!(
        "ingest done: inserted/updated={} skipped={} failed={}  mtime_filtered={}  daily_days={}  load_metrics_rows={}",
        inserted, skipped, failed, filtered_out, days_n, metrics_n
    );
    if let Some(w) = wm_now {
        println!("last_ingest_at: {}", w);
    }
    println!("db: {}", db.display());
    Ok(())
}

/// Current UTC time as `YYYY-MM-DD HH:MM:SS` (SQLite-friendly).
fn sqlite_utcnow() -> Result<String, String> {
    use std::time::{
        SystemTime,
        UNIX_EPOCH,
    };
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;
    // Format without chrono: decompose unix → UTC civil (inverse of civil_to_unix days)
    Ok(unix_to_sqlite_utc(secs))
}

fn unix_to_sqlite_utc(secs: i64) -> String {
    let days = secs.div_euclid(86400);
    let sod = secs.rem_euclid(86400);
    let h = sod / 3600;
    let mi = (sod % 3600) / 60;
    let se = sod % 60;
    // days → civil (Howard Hinnant)
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, mi, se)
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

/// Parse `$VELOFIT_HOME/.env` into a map. Does **not** mutate process env.
/// Shell/CI exports still take priority via [`env_or_dotenv`].
/// Supports `#` comments, blanks, `export ` prefixes, and quoted values.
/// Never prints secret values.
#[cfg(feature = "email")]
fn parse_velofit_dotenv() -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let path = default_velofit_root().join(".env");
    let Ok(text) = fs::read_to_string(&path) else {
        return out;
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let mut val = val.trim();
        if (val.starts_with('"') && val.ends_with('"') && val.len() >= 2)
            || (val.starts_with('\'') && val.ends_with('\'') && val.len() >= 2)
        {
            val = &val[1..val.len() - 1];
        }
        // Gmail shows app passwords as "xxxx xxxx xxxx xxxx"; IMAP wants the 16
        // alphanumerics. Strip whitespace for password keys only.
        let val = if key.contains("PASSWORD") || key.contains("SECRET") || key.contains("TOKEN") {
            val.chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
        } else {
            val.to_string()
        };
        out.insert(key.to_string(), val);
    }
    if !out.is_empty() {
        eprintln!(
            "loaded {} key(s) from {} (process env still wins when set)",
            out.len(),
            path.display()
        );
    }
    out
}

/// Prefer process environment; fall back to a key from the dotenv map.
/// Password-like keys have whitespace stripped (Gmail app-password formatting).
#[cfg(feature = "email")]
fn env_or_dotenv(key: &str, dotenv: &std::collections::HashMap<String, String>) -> Option<String> {
    let strip_ws = key.contains("PASSWORD") || key.contains("SECRET") || key.contains("TOKEN");
    let normalize = |s: String| -> String {
        if strip_ws {
            s.chars().filter(|c| !c.is_whitespace()).collect()
        } else {
            s
        }
    };
    env::var(key)
        .ok()
        .filter(|s| !s.is_empty())
        .map(normalize)
        .or_else(|| {
            dotenv
                .get(key)
                .cloned()
                .filter(|s| !s.is_empty())
                .map(normalize)
        })
}

/// Build [`email::ImapConfig`] from process env, then dotenv fallbacks.
#[cfg(feature = "email")]
fn imap_config_from_env_and_dotenv(
    dotenv: &std::collections::HashMap<String, String>,
) -> email::ImapConfig {
    let mut cfg = email::ImapConfig::default();
    if let Some(h) = env_or_dotenv(email::SYMLOAD_IMAP_HOST_ENV, dotenv) {
        cfg.host = h;
    }
    if let Some(p) = env_or_dotenv(email::SYMLOAD_IMAP_PORT_ENV, dotenv) {
        if let Ok(n) = p.trim().parse::<u16>() {
            if n > 0 {
                cfg.port = n;
            }
        }
    }
    if let Some(m) = env_or_dotenv(email::SYMLOAD_IMAP_MAILBOX_ENV, dotenv) {
        cfg.mailbox = m;
    }
    cfg
}

#[cfg(feature = "email")]
fn handle_email_fetch(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    // Reproducible local config: $VELOFIT_HOME/.env (gitignored), independent of any AI/MCP session.
    let dotenv = parse_velofit_dotenv();
    let root = default_velofit_root();

    let user = env_or_dotenv("SYMLOAD_USER", &dotenv).ok_or_else(|| {
        format!(
            "set SYMLOAD_USER (IMAP username) via env or {}/.env — do not commit credentials",
            root.display()
        )
    })?;
    let pass = env_or_dotenv("SYMLOAD_APP_PASSWORD", &dotenv).ok_or_else(|| {
        format!(
            "set SYMLOAD_APP_PASSWORD via env or {}/.env — do not commit",
            root.display()
        )
    })?;

    // Safe preflight (no secrets): helps diagnose AUTHENTICATIONFAILED without dumping .env.
    {
        let domain = user
            .rsplit_once('@')
            .map(|(_, d)| d)
            .unwrap_or("(no @ in SYMLOAD_USER)");
        eprintln!(
            "imap preflight: user_domain={} user_len={} pass_len={} (spaces already stripped)",
            domain,
            user.len(),
            pass.len()
        );
        if pass.len() != 16 {
            eprintln!(
                "warning: Gmail app passwords are usually 16 characters after removing spaces; got {}",
                pass.len()
            );
        }
    }

    let imap_cfg = imap_config_from_env_and_dotenv(&dotenv);

    // Optional IMAP SEARCH query. Default matches SRM PC8 export subjects.
    // Examples: --query "SUBJECT SRM UNSEEN"
    //           --query "OR SUBJECT SRM SUBJECT Polar"
    let query = parse_flag_value(args, "--query")
        .or_else(|| parse_flag_value(args, "-q"))
        .unwrap_or_else(|| "SUBJECT SRM".into());

    // Target directory: first non-flag positional after the command, else inbox.
    // Usage: symload email fetch [target_dir] [--query "..."]
    let target = parse_email_target_dir(args);

    let saved = email::fetch_fit_attachments_with_config(&user, &pass, &target, &query, &imap_cfg)?;

    println!(
        "Fetched {} new .fit file(s) to {}  (query: {}; imap: {}:{}/{})",
        saved.len(),
        target.display(),
        query,
        imap_cfg.host,
        imap_cfg.port,
        imap_cfg.mailbox
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
  symload ingest [path|dir] [--db PATH] [--ftp 280] [--all] [--force]
      Default: only files with mtime >= last_ingest_at (catalog_meta watermark).
      --all / -a     recheck every file (hash skip still applies)
      --force / -F   re-score all candidates (implies full scan; ignores watermark)
      FTP for each ride comes from ftp_history when set; --ftp is fallback only.
  symload reprocess [path|dir] [--ftp 280]   same as ingest --force (re-score loads)
  symload ftp list
  symload ftp set --date YYYY-MM-DD --ftp N [--sport cycling] [--source manual] [--until DATE]
  symload email fetch [target_dir] [--query "SUBJECT SRM"]
      default dir: $VELOFIT_HOME/inbox; default query matches SRM exports
      loads $VELOFIT_HOME/.env when present (does not override existing env)
  symload inbox promote [--from DIR] [--to DIR]

Environment (never commit secrets; email also reads $VELOFIT_HOME/.env):
  VELOFIT_HOME          archive root (default: ~/velofit)
  SYMLOAD_DB            SQLite path override
  SYMLOAD_USER          IMAP username for email fetch
  SYMLOAD_APP_PASSWORD  IMAP app password
  SYMLOAD_IMAP_HOST     IMAP host (default: imap.gmail.com)
  SYMLOAD_IMAP_PORT     IMAP TLS port (default: 993)
  SYMLOAD_IMAP_MAILBOX  mailbox to search (default: INBOX)
  SYMLOAD_INGEST_VERBOSE  set to log skipped files

Privacy: the catalog and FIT archive live under VELOFIT_HOME only — not in the SymWorx repo.
"#
    );
}
