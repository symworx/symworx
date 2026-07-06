// symload — headless CLI for activity ingestion and symworx-loadsym tooling.
//
// This binary is provided by the symworx-loadsym crate.
//
// Usage examples:
//   symload stats ride.fit --ftp 280
//   symload stats ./inbox/
//   symload db print-schema
//   cargo build -p symworx-loadsym --features email
//   symload email fetch ~/symload/inbox

use std::{
    env,
    path::{Path, PathBuf},
};

#[cfg(feature = "email")]
use symworx_io::email;
use symworx_io::load_activity;
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
        eprintln!("DB schema support requires the 'db' feature (or build without feature gating)");
    }
}

#[cfg(feature = "email")]
fn handle_email_fetch(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let user = env::var("SYMLOAD_USER")?;
    let pass = env::var("SYMLOAD_APP_PASSWORD")?;

    let target = if let Some(d) = args.get(2) {
        PathBuf::from(d)
    } else {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join("symload").join("inbox")
    };

    let saved = email::fetch_srm_fit_attachments(&user, &pass, &target)?;

    println!("Fetched {} files to {}", saved.len(), target.display());
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
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_file() {
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("fit") {
                        out.push(p);
                    }
                }
            }
        }
    }
    out.sort();
    out
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

    println!(
        "JSON: {{\"file\":\"{}\",\"duration_s\":{:.1},\"avg_w\":{:.1},\"max_w\":{:.0},\"np\":{:.0},\"if\":{:.2},\"tss\":{:.1},\"ftp\":{:.0}}}",
        path.display(),
        dur,
        avg,
        maxp,
        m.np,
        m.if_,
        m.tss,
        ftp
    );

    Ok(())
}

fn print_usage() {
    eprintln!(
        r#"symload — headless activity ingest / stats (symworx-loadsym)

Commands:
  symload stats <file.fit> [--ftp 280] [--json]
  symload stats <directory>
  symload db print-schema [--dialect postgres|sqlite]
  symload email fetch [target_dir]   (requires --features email on symworx-loadsym)

The recommended location for your data is ~/symload/inbox and your DB project at ~/symload.
"#
    );
}
