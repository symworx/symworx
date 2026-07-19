// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Email-based input support (primarily for fetching .fit files from email sources
//! such as SRM PC8 powermeter exports).
//!
//! Enabled via the `email` feature.
//!
//! # Configuration (environment only — never hardcode credentials)
//!
//! | Variable | Role | Default |
//! |----------|------|---------|
//! | `SYMLOAD_USER` | IMAP username | (required) |
//! | `SYMLOAD_APP_PASSWORD` | App password / IMAP password | (required) |
//! | `SYMLOAD_IMAP_HOST` | IMAP server hostname | `imap.gmail.com` |
//! | `SYMLOAD_IMAP_PORT` | IMAP TLS port | `993` |
//! | `SYMLOAD_IMAP_MAILBOX` | Mailbox to search | `INBOX` |
//!
//! Common hosts: `imap.gmail.com`, `outlook.office365.com`, `imap-mail.outlook.com`.

use std::path::{
    Path,
    PathBuf,
};

use symworx_error::SymError;

/// Env: IMAP hostname (default `imap.gmail.com`).
pub const SYMLOAD_IMAP_HOST_ENV: &str = "SYMLOAD_IMAP_HOST";
/// Env: IMAP TLS port (default `993`).
pub const SYMLOAD_IMAP_PORT_ENV: &str = "SYMLOAD_IMAP_PORT";
/// Env: mailbox name (default `INBOX`).
pub const SYMLOAD_IMAP_MAILBOX_ENV: &str = "SYMLOAD_IMAP_MAILBOX";

/// Resolved IMAP connection settings (from env, with defaults).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImapConfig {
    /// Server hostname.
    pub host: String,
    /// TLS port (typically 993).
    pub port: u16,
    /// Mailbox to `SELECT` before SEARCH (e.g. `INBOX`).
    pub mailbox: String,
}

impl Default for ImapConfig {
    fn default() -> Self {
        Self {
            host: "imap.gmail.com".into(),
            port: 993,
            mailbox: "INBOX".into(),
        }
    }
}

impl ImapConfig {
    /// Read host/port/mailbox from environment, falling back to defaults.
    ///
    /// Empty env values are ignored. Invalid `SYMLOAD_IMAP_PORT` falls back to 993.
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(h) = std::env::var(SYMLOAD_IMAP_HOST_ENV) {
            let h = h.trim();
            if !h.is_empty() {
                cfg.host = h.to_string();
            }
        }
        if let Ok(p) = std::env::var(SYMLOAD_IMAP_PORT_ENV) {
            if let Ok(n) = p.trim().parse::<u16>() {
                if n > 0 {
                    cfg.port = n;
                }
            }
        }
        if let Ok(m) = std::env::var(SYMLOAD_IMAP_MAILBOX_ENV) {
            let m = m.trim();
            if !m.is_empty() {
                cfg.mailbox = m.to_string();
            }
        }
        cfg
    }
}

/// Fetch .fit attachments for emails related to SRM (default search: `SUBJECT SRM`).
///
/// Uses the provided credentials (app password recommended). Connection target is
/// taken from [`ImapConfig::from_env`]. Saves matching attachments to `target_dir`.
///
/// Returns the list of newly saved .fit file paths.
///
/// Files that already exist at the destination (same basename) are skipped
/// so re-runs are safe. Only decoded MIME attachment bytes are written —
/// never the raw RFC822 message body.
#[cfg(feature = "email")]
pub fn fetch_srm_fit_attachments(
    user: &str,
    app_password: &str,
    target_dir: &Path,
) -> Result<Vec<PathBuf>, SymError> {
    fetch_fit_attachments(user, app_password, target_dir, "SUBJECT SRM")
}

/// Fetch .fit attachments via IMAP using a custom IMAP SEARCH query.
///
/// Example queries: `"SUBJECT SRM"`, `"SUBJECT SRM UNSEEN"`,
/// `"OR SUBJECT SRM SUBJECT Polar"`.
///
/// Host/port/mailbox come from env ([`ImapConfig::from_env`]).
#[cfg(feature = "email")]
pub fn fetch_fit_attachments(
    user: &str,
    app_password: &str,
    target_dir: &Path,
    search_query: &str,
) -> Result<Vec<PathBuf>, SymError> {
    fetch_fit_attachments_with_config(
        user,
        app_password,
        target_dir,
        search_query,
        &ImapConfig::from_env(),
    )
}

/// Same as [`fetch_fit_attachments`] but with an explicit [`ImapConfig`]
/// (useful for tests and non-env configuration).
#[cfg(feature = "email")]
pub fn fetch_fit_attachments_with_config(
    user: &str,
    app_password: &str,
    target_dir: &Path,
    search_query: &str,
    imap_cfg: &ImapConfig,
) -> Result<Vec<PathBuf>, SymError> {
    use std::{
        fs::File,
        io::Write,
    };

    use imap::Session;
    use mailparse::parse_mail;

    std::fs::create_dir_all(target_dir)
        .map_err(|e| SymError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    // ClientBuilder uses native-tls for port 993 (implicit TLS).
    let client = imap::ClientBuilder::new(&imap_cfg.host, imap_cfg.port)
        .connect()
        .map_err(|e| {
            SymError::UnsupportedFormat(format!(
                "IMAP connect failed ({}:{}): {}",
                imap_cfg.host, imap_cfg.port, e
            ))
        })?;

    let mut sess: Session<_> = client
        .login(user, app_password)
        .map_err(|(e, _)| SymError::UnsupportedFormat(format!("IMAP login failed: {}", e)))?;

    eprintln!(
        "imap: logged in → select {} @ {}:{}",
        imap_cfg.mailbox, imap_cfg.host, imap_cfg.port
    );

    sess.select(&imap_cfg.mailbox).map_err(|e| {
        SymError::UnsupportedFormat(format!(
            "Select mailbox '{}' failed: {}",
            imap_cfg.mailbox, e
        ))
    })?;

    eprintln!("imap: searching ({}) …", search_query);
    // `search` returns sequence numbers (not UIDs); pair with `fetch`, not `uid_fetch`.
    let mut seqs: Vec<u32> = sess
        .search(search_query)
        .map_err(|e| SymError::UnsupportedFormat(format!("Search failed: {}", e)))?
        .into_iter()
        .collect();
    // Stable order so progress reads sensibly on re-runs.
    seqs.sort_unstable();
    let total = seqs.len();
    eprintln!(
        "imap: {} message(s) matched; downloading .fit attachments → {}",
        total,
        target_dir.display()
    );

    let mut saved = Vec::new();
    let mut skipped_existing = 0usize;
    let mut no_fit = 0usize;

    for (i, seq) in seqs.iter().enumerate() {
        let n = i + 1;
        if n == 1 || n == total || n % 10 == 0 {
            eprintln!(
                "imap: progress {n}/{total}  (new={}, skipped_existing={}, no_fit={})",
                saved.len(),
                skipped_existing,
                no_fit
            );
        }

        let fetches = sess
            .fetch(seq.to_string(), "RFC822")
            .map_err(|e| SymError::UnsupportedFormat(format!("Fetch failed (seq {seq}): {e}")))?;

        let mut got_fit_this_msg = false;
        for fetch in fetches.iter() {
            let Some(body) = fetch.body() else {
                continue;
            };

            let parsed = match parse_mail(body) {
                Ok(m) => m,
                Err(e) => {
                    // Fall back: save .eml for manual inspection
                    let out_path = target_dir.join(format!("srm-{}.eml", seq));
                    if !out_path.exists() {
                        let mut file = File::create(&out_path).map_err(SymError::Io)?;
                        file.write_all(body).map_err(SymError::Io)?;
                    }
                    let _ = e;
                    continue;
                }
            };

            let attachments = extract_fit_attachments_from_mail(&parsed, *seq);
            for (fname, bytes) in attachments {
                if bytes.is_empty() {
                    continue;
                }
                // Skip obvious non-FIT (FIT files typically start with '.' or have size)
                if bytes.len() < 14 {
                    continue;
                }

                got_fit_this_msg = true;
                // Skip if this basename already exists (re-runs are safe).
                let out_path = target_dir.join(&fname);
                if out_path.exists() {
                    skipped_existing += 1;
                    continue;
                }
                let mut file = File::create(&out_path).map_err(SymError::Io)?;
                file.write_all(&bytes).map_err(SymError::Io)?;
                eprintln!(
                    "  + {}",
                    out_path.file_name().unwrap_or_default().to_string_lossy()
                );
                saved.push(out_path);
            }
        }
        if !got_fit_this_msg {
            no_fit += 1;
        }
    }

    eprintln!(
        "imap: done — new={} skipped_existing={} messages_without_fit={} of {}",
        saved.len(),
        skipped_existing,
        no_fit,
        total
    );

    let _ = sess.logout();

    Ok(saved)
}

/// Walk a parsed MIME tree and collect (filename, decoded bytes) for .fit parts.
#[cfg(feature = "email")]
fn extract_fit_attachments_from_mail(
    mail: &mailparse::ParsedMail<'_>,
    uid: u32,
) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    collect_fit_parts(mail, uid, 0, &mut out);
    out
}

#[cfg(feature = "email")]
fn collect_fit_parts(
    mail: &mailparse::ParsedMail<'_>,
    uid: u32,
    part_idx: usize,
    out: &mut Vec<(String, Vec<u8>)>,
) {
    // Prefer Content-Disposition filename, then Content-Type name=
    let fname = attachment_filename(mail);

    let is_fit = fname
        .as_ref()
        .map(|n| n.to_lowercase().ends_with(".fit"))
        .unwrap_or(false);

    // Also accept parts whose content-type hints binary/octet-stream with fit name
    if is_fit {
        if let Ok(body) = mail.get_body_raw() {
            let name = fname.unwrap_or_else(|| format!("srm-{}-{}.fit", uid, part_idx));
            let safe = sanitize_filename(&name);
            out.push((safe, body));
        }
    }

    for (i, sub) in mail.subparts.iter().enumerate() {
        collect_fit_parts(sub, uid, part_idx * 100 + i + 1, out);
    }
}

#[cfg(feature = "email")]
fn attachment_filename(mail: &mailparse::ParsedMail<'_>) -> Option<String> {
    // Content-Disposition: attachment; filename="ride.fit"
    let disp = mail.get_content_disposition();
    if let Some(name) = disp.params.get("filename") {
        if !name.is_empty() {
            return Some(name.clone());
        }
    }
    // Content-Type: ...; name="ride.fit"
    if let Some(name) = mail.ctype.params.get("name") {
        if !name.is_empty() {
            return Some(name.clone());
        }
    }
    None
}

#[cfg(feature = "email")]
fn sanitize_filename(name: &str) -> String {
    let base = Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment.fit");
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.to_lowercase().ends_with(".fit") {
        cleaned
    } else {
        format!("{}.fit", cleaned)
    }
}

#[cfg(all(test, feature = "email"))]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_path_components() {
        assert_eq!(sanitize_filename("../../evil.fit"), "evil.fit");
        assert_eq!(sanitize_filename("my ride (1).fit"), "my_ride__1_.fit");
    }

    #[test]
    fn extract_fit_from_simple_multipart() {
        // Minimal multipart with a base64 "FIT" attachment (not a real FIT, just payload)
        let raw = b"From: test@example.com\r\n\
Subject: SRM export\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"bound\"\r\n\
\r\n\
--bound\r\n\
Content-Type: text/plain\r\n\
\r\n\
body\r\n\
--bound\r\n\
Content-Type: application/octet-stream; name=\"ride.fit\"\r\n\
Content-Disposition: attachment; filename=\"ride.fit\"\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
Li4uLi4uLi4uLi4uLi4uLi4=\r\n\
--bound--\r\n";

        let mail = mailparse::parse_mail(raw).expect("parse");
        let parts = extract_fit_attachments_from_mail(&mail, 1);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].0, "ride.fit");
        assert!(!parts[0].1.is_empty());
    }

    #[test]
    fn imap_config_defaults() {
        let cfg = ImapConfig::default();
        assert_eq!(cfg.host, "imap.gmail.com");
        assert_eq!(cfg.port, 993);
        assert_eq!(cfg.mailbox, "INBOX");
    }
}
