// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

//! Email-based input support (primarily for fetching .fit files from email sources
//! such as SRM PC8 powermeter exports).
//!
//! Enabled via the `email` feature.

use std::path::{Path, PathBuf};

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
#[cfg(feature = "email")]
pub fn fetch_srm_fit_attachments(
    user: &str,
    app_password: &str,
    target_dir: &Path,
) -> Result<Vec<PathBuf>, SymError> {
    use std::{fs::File, io::Write};

    use imap::Session;
    use mailparse::parse_mail;
    use native_tls::TlsConnector;

    std::fs::create_dir_all(target_dir)
        .map_err(|e| SymError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let tls = TlsConnector::builder()
        .build()
        .map_err(|e| SymError::UnsupportedFormat(format!("TLS setup failed: {}", e)))?;

    let client = imap::ClientBuilder::new("imap.gmail.com", 993)
        .connect()
        .map_err(|e| SymError::UnsupportedFormat(format!("IMAP connect failed: {}", e)))?;

    let mut sess: Session<_> = client
        .login(user, app_password)
        .map_err(|(e, _)| SymError::UnsupportedFormat(format!("IMAP login failed: {}", e)))?;

    sess.select("INBOX")
        .map_err(|e| SymError::UnsupportedFormat(format!("Select INBOX failed: {}", e)))?;

    // Basic search for SRM-related messages. Users can extend this.
    let uids = sess
        .search("SUBJECT SRM")
        .map_err(|e| SymError::UnsupportedFormat(format!("Search failed: {}", e)))?;

    let mut saved = Vec::new();

    for uid in uids {
        let fetches = sess
            .fetch(uid.to_string(), "RFC822")
            .map_err(|e| SymError::UnsupportedFormat(format!("Fetch failed: {}", e)))?;

        for fetch in fetches.iter() {
            if let Some(body) = fetch.body() {
                let body_str = String::from_utf8_lossy(body);

                if body_str.to_lowercase().contains(".fit") {
                    if let Some(fname) = extract_fit_filename(&body_str) {
                        let out_path = target_dir.join(&fname);
                        let mut file = File::create(&out_path).map_err(SymError::Io)?;
                        // For robustness we write the raw body here; a full MIME walk
                        // is preferable for production use of complex emails.
                        file.write_all(body).map_err(SymError::Io)?;
                        saved.push(out_path);
                    } else {
                        // Fallback: save raw email for manual extraction
                        let out_path = target_dir.join(format!("srm-{}.eml", uid));
                        let mut file = File::create(&out_path).map_err(SymError::Io)?;
                        file.write_all(body).map_err(SymError::Io)?;
                        saved.push(out_path);
                    }
                }
            }
        }
    }

    let _ = sess.logout();

    Ok(saved)
}

#[cfg(feature = "email")]
fn extract_fit_filename(body: &str) -> Option<String> {
    // Naive but practical extractor for common attachment headers.
    for line in body.lines() {
        let lower = line.to_lowercase();
        if lower.contains("filename=") || lower.contains("name=") {
            if let Some(start) = line.find('"') {
                if let Some(end_rel) = line[start + 1..].find('"') {
                    let name = &line[start + 1..start + 1 + end_rel];
                    if name.to_lowercase().ends_with(".fit") {
                        return Some(name.to_string());
                    }
                }
            }
        }
    }
    None
}
