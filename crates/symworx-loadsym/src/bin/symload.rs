// symload — headless CLI for activity ingestion and symworx-loadsym tooling.
//
// This binary is provided by the symworx-loadsym crate.
//
// Usage examples:
//   symload stats ride.fit --ftp 280
//   symload stats ~/velofit/raw
//   symload db print-schema
//   cargo build -p symworx-loadsym --features "fit,email,db"
//   symload email fetch ~/velofit/inbox
//   symload inbox promote

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
            handle_db_command(&args);
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

fn handle_db_command(args: &[String]) {
    #[cfg(feature = "db")]
    {
        if args.len() > 2 && (args[2] == "print-schema" || args[2] == "schema") {
            let dialect = if args.iter().any(|a| a == "--sqlite" || a == "sqlite") {
                "sqlite"
            } else {
                "postgres"
            };
            println!("{}", symworx_loadsym_db::get_schema(dialect));
        } else {
            eprintln!("symload db print-schema [--dialect postgres|sqlite]");
        }
    }
    #[cfg(not(feature = "db"))]
    {
        let _ = args;
        eprintln!("DB schema support requires the 'db' feature");
    }
}

#[cfg(feature = "email")]
fn handle_email_fetch(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let user = env::var("SYMLOAD_USER")?;
    let pass = env::var("SYMLOAD_APP_PASSWORD")?;

    let target = if let Some(d) = args.get(2) {
        if d.starts_with('-') {
            default_velofit_inbox()
        } else {
            PathBuf::from(d)
        }
    } else {
        default_velofit_inbox()
    };

    let saved = email::fetch_srm_fit_attachments(&user, &pass, &target)?;

    println!(
        "Fetched {} new .fit file(s) to {}",
        saved.len(),
        target.display()
    );
    for p in &saved {
        println!("  {}", p.display());
    }
    if saved.is_empty() {
        println!("(none new — already present or no matching attachments)");
    }
    Ok(())
}

/// Move unique .fit files from inbox → raw (skip if destination exists with same size).
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
                // Already archived; remove inbox copy to avoid reprocessing
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
        fs::rename(p, &dest).or_else(|_| {
            // Cross-device: copy then remove
            fs::copy(p, &dest).and_then(|_| fs::remove_file(p))
        })?;
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
    if moved > 0 {
        println!("Next: syncd velofit   # push ~/velofit to S3");
    }
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
        r#"symload — headless activity ingest / stats (symworx-loadsym)

Commands:
  symload stats <file.fit | dir> [--ftp 280] [--json]
  symload db print-schema [--dialect postgres|sqlite]
  symload email fetch [target_dir]     (default: ~/velofit/inbox; needs --features email)
  symload inbox promote [--from DIR] [--to DIR]
      Move unique .fit from inbox → raw (defaults: ~/velofit/inbox → ~/velofit/raw)

Personal archive layout:
  ~/velofit/inbox   email / manual drop
  ~/velofit/raw     S3-mirrored archive (s3:bitterbeta-useast1-velofit)
  Override root with VELOFIT_HOME.

Env (email):
  SYMLOAD_USER              Gmail address (e.g. nberry.fitdata@gmail.com)
  SYMLOAD_APP_PASSWORD      Gmail App Password

After promote:  syncd velofit
"#
    );
}
