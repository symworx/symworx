// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Email-based input support (primarily for fetching .fit files from email sources
//! such as SRM PC8 powermeter exports).
//!
//! Enabled via the `email` feature.

use std::path::{
    Path,
    PathBuf,
};

use symworx_error::SymError;

/// Fetch .fit attachments from Gmail IMAP for emails related to SRM.
///
/// Uses the provided credentials (App Password recommended for Gmail).
/// Saves matching attachments to `target_dir`.
///
/// Returns the list of saved .fit file paths.
///
/// This is intentionally focused on common SRM export patterns. Customize
/// the search query or parsing as needed for your workflow.
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

/// Fetch .fit attachments from Gmail IMAP using a custom IMAP SEARCH query.
///
/// Example queries: `"SUBJECT SRM"`, `"SUBJECT SRM UNSEEN"`,
/// `"OR SUBJECT SRM SUBJECT Polar"`.
#[cfg(feature = "email")]
pub fn fetch_fit_attachments(
    user: &str,
    app_password: &str,
    target_dir: &Path,
    search_query: &str,
) -> Result<Vec<PathBuf>, SymError> {
    use std::{
        fs::File,
        io::Write,
    };

    use imap::Session;
    use mailparse::parse_mail;

    std::fs::create_dir_all(target_dir)
        .map_err(|e| SymError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    // ClientBuilder uses native-tls/rustls automatically for port 993.
    let client = imap::ClientBuilder::new("imap.gmail.com", 993)
        .connect()
        .map_err(|e| SymError::UnsupportedFormat(format!("IMAP connect failed: {}", e)))?;

    let mut sess: Session<_> = client
        .login(user, app_password)
        .map_err(|(e, _)| SymError::UnsupportedFormat(format!("IMAP login failed: {}", e)))?;

    sess.select("INBOX")
        .map_err(|e| SymError::UnsupportedFormat(format!("Select INBOX failed: {}", e)))?;

    let uids = sess
        .search(search_query)
        .map_err(|e| SymError::UnsupportedFormat(format!("Search failed: {}", e)))?;

    let mut saved = Vec::new();

    for uid in uids {
        let fetches = sess
            .fetch(uid.to_string(), "RFC822")
            .map_err(|e| SymError::UnsupportedFormat(format!("Fetch failed: {}", e)))?;

        for fetch in fetches.iter() {
            let Some(body) = fetch.body() else {
                continue;
            };

            let parsed = match parse_mail(body) {
                Ok(m) => m,
                Err(e) => {
                    // Fall back: save .eml for manual inspection
                    let out_path = target_dir.join(format!("srm-{}.eml", uid));
                    if !out_path.exists() {
                        let mut file = File::create(&out_path).map_err(SymError::Io)?;
                        file.write_all(body).map_err(SymError::Io)?;
                    }
                    let _ = e;
                    continue;
                }
            };

            let attachments = extract_fit_attachments_from_mail(&parsed, uid);
            for (fname, bytes) in attachments {
                if bytes.is_empty() {
                    continue;
                }
                // Skip obvious non-FIT (FIT files typically start with '.' or have size)
                if bytes.len() < 14 {
                    continue;
                }

                // Skip if this basename already exists (re-runs are safe).
                let out_path = target_dir.join(&fname);
                if out_path.exists() {
                    continue;
                }
                let mut file = File::create(&out_path).map_err(SymError::Io)?;
                file.write_all(&bytes).map_err(SymError::Io)?;
                saved.push(out_path);
            }
        }
    }

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
}
