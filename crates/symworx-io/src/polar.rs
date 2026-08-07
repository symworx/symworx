// Copyright (c) 2026 PalEm Dynamics LLC
// Licensed under the Apache License, Version 2.0.

//! Polar AccessLink client for personal LoadSym ingestion.
//!
//! Enabled via the `polar` feature. Credentials and tokens live under
//! `$VELOFIT_HOME` (never in the SymWorx repo).
//!
//! # Environment (never commit secrets)
//!
//! | Variable | Role |
//! |----------|------|
//! | `POLAR_CLIENT_ID` | OAuth client id from admin.polaraccesslink.com |
//! | `POLAR_CLIENT_SECRET` | OAuth client secret |
//! | `POLAR_ACCESS_TOKEN` | User bearer token (set by `symload polar auth`) |
//! | `POLAR_USER_ID` | Polar user id (`x_user_id` from token response) |
//! | `POLAR_REDIRECT_URI` | Must match client registration (default `http://127.0.0.1:8765/callback`) |
//! | `POLAR_MEMBER_ID` | Partner member-id for register (default `local-symload`) |
//!
//! Token file (preferred after auth): `$VELOFIT_HOME/polar_token.json`.
//!
//! API docs: <https://www.polar.com/accesslink-api/>

use std::{
    fs::{
        self,
        File,
    },
    io::{
        Read,
        Write,
    },
    net::TcpListener,
    path::{
        Path,
        PathBuf,
    },
    time::Duration,
};

use serde::{
    Deserialize,
    Serialize,
};
use symworx_error::SymError;

use crate::paths::default_velofit_root;

/// Env: OAuth client id.
pub const POLAR_CLIENT_ID_ENV: &str = "POLAR_CLIENT_ID";
/// Env: OAuth client secret.
pub const POLAR_CLIENT_SECRET_ENV: &str = "POLAR_CLIENT_SECRET";
/// Env: user access token.
pub const POLAR_ACCESS_TOKEN_ENV: &str = "POLAR_ACCESS_TOKEN";
/// Env: Polar user id.
pub const POLAR_USER_ID_ENV: &str = "POLAR_USER_ID";
/// Env: OAuth redirect URI.
pub const POLAR_REDIRECT_URI_ENV: &str = "POLAR_REDIRECT_URI";
/// Env: register member-id.
pub const POLAR_MEMBER_ID_ENV: &str = "POLAR_MEMBER_ID";

/// Default local OAuth callback (register this exact URL on admin.polaraccesslink.com).
pub const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:8765/callback";
/// Default partner member-id for personal use.
pub const DEFAULT_MEMBER_ID: &str = "local-symload";

const AUTH_URL: &str = "https://flow.polar.com/oauth2/authorization";
const TOKEN_URL: &str = "https://polarremote.com/v2/oauth2/token";
const API_BASE: &str = "https://www.polaraccesslink.com/v3";

/// Default download directory: `$VELOFIT_HOME/raw/polar`.
pub fn default_polar_raw_dir() -> PathBuf {
    default_velofit_root().join("raw").join("polar")
}

/// Path for persisted token JSON: `$VELOFIT_HOME/polar_token.json`.
pub fn default_token_path() -> PathBuf {
    default_velofit_root().join("polar_token.json")
}

/// OAuth client + optional user token for AccessLink.
#[derive(Debug, Clone)]
pub struct PolarCredentials {
    /// OAuth client id.
    pub client_id: String,
    /// OAuth client secret.
    pub client_secret: String,
    /// Bearer access token (after auth).
    pub access_token: Option<String>,
    /// Polar ecosystem user id.
    pub user_id: Option<i64>,
    /// Redirect URI registered with the client.
    pub redirect_uri: String,
    /// Member-id used when registering the user with AccessLink.
    pub member_id: String,
}

/// Persisted token payload (no client secret).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolarTokenFile {
    /// Bearer token.
    pub access_token: String,
    /// Polar user id from token response.
    pub user_id: i64,
    /// Client id that obtained the token (informational).
    #[serde(default)]
    pub client_id: Option<String>,
    /// When the file was written (ISO-ish string).
    #[serde(default)]
    pub saved_at: Option<String>,
}

/// Summary of one AccessLink exercise (list endpoint).
#[derive(Debug, Clone)]
pub struct PolarExerciseSummary {
    /// Hashed exercise id used in FIT download URL.
    pub id: String,
    /// Start time string from API (local or ISO; used for filenames).
    pub start_time: Option<String>,
    /// Upload time if present.
    pub upload_time: Option<String>,
    /// Sport label when present.
    pub sport: Option<String>,
    /// Device name when present.
    pub device: Option<String>,
}

/// Result of a fetch run.
#[derive(Debug, Clone, Default)]
pub struct PolarFetchReport {
    /// Newly written FIT paths.
    pub saved: Vec<PathBuf>,
    /// Already present (skipped).
    pub skipped_existing: usize,
    /// FIT download HTTP failures.
    pub failed: usize,
    /// Exercises listed by API.
    pub listed: usize,
}

impl PolarCredentials {
    /// Build from explicit strings (CLI after dotenv merge).
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: Option<String>,
        member_id: Option<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            access_token: None,
            user_id: None,
            redirect_uri: redirect_uri
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| DEFAULT_REDIRECT_URI.into()),
            member_id: member_id
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| DEFAULT_MEMBER_ID.into()),
        }
    }

    /// Load credentials from process env (and optional token file under `$VELOFIT_HOME`).
    ///
    /// Does not read `.env` itself — the CLI merges dotenv into env lookups first.
    pub fn from_env() -> Result<Self, SymError> {
        let client_id = env_nonempty(POLAR_CLIENT_ID_ENV).ok_or_else(|| {
            SymError::UnsupportedFormat(format!(
                "set {POLAR_CLIENT_ID_ENV} (from https://admin.polaraccesslink.com)"
            ))
        })?;
        let client_secret = env_nonempty(POLAR_CLIENT_SECRET_ENV).ok_or_else(|| {
            SymError::UnsupportedFormat(format!(
                "set {POLAR_CLIENT_SECRET_ENV} (from https://admin.polaraccesslink.com)"
            ))
        })?;
        let mut creds = Self::new(
            client_id,
            client_secret,
            env_nonempty(POLAR_REDIRECT_URI_ENV),
            env_nonempty(POLAR_MEMBER_ID_ENV),
        );
        if let Some(tok) = env_nonempty(POLAR_ACCESS_TOKEN_ENV) {
            creds.access_token = Some(tok);
        }
        if let Some(uid) = env_nonempty(POLAR_USER_ID_ENV) {
            if let Ok(n) = uid.parse::<i64>() {
                creds.user_id = Some(n);
            }
        }
        // Token file fills gaps / overrides empty env.
        if let Ok(file) = load_token_file(&default_token_path()) {
            if creds.access_token.is_none() {
                creds.access_token = Some(file.access_token);
            }
            if creds.user_id.is_none() {
                creds.user_id = Some(file.user_id);
            }
        }
        Ok(creds)
    }

    /// True when a user access token is available.
    pub fn has_user_token(&self) -> bool {
        self.access_token.as_ref().map(|t| !t.is_empty()).unwrap_or(false)
    }

    /// Browser authorization URL (user must grant access).
    pub fn authorization_url(&self, state: &str) -> String {
        format!(
            "{AUTH_URL}?response_type=code&client_id={}&redirect_uri={}&scope=accesslink.read_all&state={}",
            urlencoding_minimal(&self.client_id),
            urlencoding_minimal(&self.redirect_uri),
            urlencoding_minimal(state),
        )
    }
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Minimal URL-encoding for query values (OAuth params).
fn urlencoding_minimal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push_str("%20"),
            b':' => out.push_str("%3A"),
            b'/' => out.push_str("%2F"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Load token JSON if present.
pub fn load_token_file(path: &Path) -> Result<PolarTokenFile, SymError> {
    let text = fs::read_to_string(path).map_err(SymError::Io)?;
    serde_json::from_str(&text).map_err(|e| SymError::UnsupportedFormat(format!("parse polar token file: {e}")))
}

/// Write token JSON (creates parent dirs). Mode is best-effort 0600 on Unix via umask only.
pub fn save_token_file(path: &Path, token: &PolarTokenFile) -> Result<(), SymError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(SymError::Io)?;
    }
    let text = serde_json::to_string_pretty(token)
        .map_err(|e| SymError::UnsupportedFormat(format!("serialize token: {e}")))?;
    let mut f = File::create(path).map_err(SymError::Io)?;
    f.write_all(text.as_bytes()).map_err(SymError::Io)?;
    f.write_all(b"\n").map_err(SymError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Exchange authorization code for access token.
pub fn exchange_authorization_code(creds: &PolarCredentials, code: &str) -> Result<PolarTokenFile, SymError> {
    let basic = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        format!("{}:{}", creds.client_id, creds.client_secret),
    );
    let body = format!(
        "grant_type=authorization_code&code={}&redirect_uri={}",
        urlencoding_minimal(code),
        urlencoding_minimal(&creds.redirect_uri),
    );
    let resp = ureq::post(TOKEN_URL)
        .set("Authorization", &format!("Basic {basic}"))
        .set("Content-Type", "application/x-www-form-urlencoded")
        .set("Accept", "application/json")
        .timeout(Duration::from_secs(60))
        .send_string(&body)
        .map_err(|e| SymError::UnsupportedFormat(format!("Polar token request failed: {e}")))?;

    let status = resp.status();
    let text = resp
        .into_string()
        .map_err(|e| SymError::UnsupportedFormat(format!("token body: {e}")))?;
    if status != 200 {
        return Err(SymError::UnsupportedFormat(format!(
            "Polar token endpoint HTTP {status}: {text}"
        )));
    }

    #[derive(Deserialize)]
    struct TokenResp {
        access_token: String,
        #[serde(default)]
        x_user_id: Option<i64>,
        #[serde(default)]
        #[allow(dead_code)]
        token_type: Option<String>,
        #[serde(default)]
        #[allow(dead_code)]
        expires_in: Option<i64>,
    }
    let tr: TokenResp =
        serde_json::from_str(&text).map_err(|e| SymError::UnsupportedFormat(format!("token JSON: {e}")))?;
    let user_id = tr
        .x_user_id
        .ok_or_else(|| SymError::UnsupportedFormat("token response missing x_user_id".into()))?;
    Ok(PolarTokenFile {
        access_token: tr.access_token,
        user_id,
        client_id: Some(creds.client_id.clone()),
        saved_at: Some(now_iso_approx()),
    })
}

/// Register the authorized user with AccessLink (required once before data access).
///
/// `409 Conflict` is treated as success (already registered).
pub fn register_user(access_token: &str, member_id: &str) -> Result<(), SymError> {
    let body = serde_json::json!({ "member-id": member_id });
    let result = ureq::post(&format!("{API_BASE}/users"))
        .set("Authorization", &format!("Bearer {access_token}"))
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .timeout(Duration::from_secs(60))
        .send_json(body);

    match result {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(409, _)) => {
            eprintln!("polar: user already registered (409) — continuing");
            Ok(())
        }
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            Err(SymError::UnsupportedFormat(format!(
                "Polar register user HTTP {code}: {text}"
            )))
        }
        Err(e) => Err(SymError::UnsupportedFormat(format!("Polar register user failed: {e}"))),
    }
}

/// Interactive OAuth: local callback server → token exchange → register user → save token file.
///
/// `open_browser`: when true, tries `xdg-open` / `open` for the authorize URL.
pub fn run_oauth_flow(
    mut creds: PolarCredentials,
    token_path: &Path,
    open_browser: bool,
) -> Result<PolarTokenFile, SymError> {
    let state = format!("symload-{}", std::process::id());
    let auth_url = creds.authorization_url(&state);
    eprintln!("polar: open this URL to authorize AccessLink:\n  {auth_url}\n");
    eprintln!("polar: waiting for redirect on {} …", creds.redirect_uri);

    if open_browser {
        try_open_browser(&auth_url);
    }

    let code = wait_for_oauth_code(&creds.redirect_uri, &state)?;
    eprintln!("polar: authorization code received — exchanging for token …");
    let token = exchange_authorization_code(&creds, &code)?;
    eprintln!("polar: registering user (member-id={}) …", creds.member_id);
    register_user(&token.access_token, &creds.member_id)?;
    save_token_file(token_path, &token)?;
    eprintln!(
        "polar: token saved to {} (mode 0600 when supported)",
        token_path.display()
    );

    creds.access_token = Some(token.access_token.clone());
    creds.user_id = Some(token.user_id);
    let _ = creds;
    Ok(token)
}

/// Parse host/port/path from redirect URI and accept one OAuth callback.
fn wait_for_oauth_code(redirect_uri: &str, expected_state: &str) -> Result<String, SymError> {
    let (host, port, path_prefix) = parse_redirect_listen(redirect_uri)?;
    let bind = format!("{host}:{port}");
    let listener = TcpListener::bind(&bind).map_err(|e| {
        SymError::UnsupportedFormat(format!(
            "bind OAuth callback {bind} failed: {e} — free the port or set POLAR_REDIRECT_URI"
        ))
    })?;
    listener
        .set_nonblocking(false)
        .map_err(|e| SymError::UnsupportedFormat(format!("listener: {e}")))?;

    // Single accept (user completes browser flow).
    let (mut stream, _) = listener
        .accept()
        .map_err(|e| SymError::UnsupportedFormat(format!("OAuth accept: {e}")))?;

    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).map_err(SymError::Io)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let first_line = req.lines().next().unwrap_or("");
    // GET /callback?code=...&state=... HTTP/1.1
    let target = first_line.split_whitespace().nth(1).unwrap_or("");
    if !target.starts_with(&path_prefix) && path_prefix != "/" {
        // still parse query if any
    }
    let query = target.split('?').nth(1).unwrap_or("");
    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    let mut error: Option<String> = None;
    for part in query.split('&') {
        if let Some((k, v)) = part.split_once('=') {
            match k {
                "code" => code = Some(url_decode_minimal(v)),
                "state" => state = Some(url_decode_minimal(v)),
                "error" => error = Some(url_decode_minimal(v)),
                _ => {}
            }
        }
    }

    let body = if let Some(ref err) = error {
        format!("<html><body><h1>Polar auth failed</h1><p>{err}</p><p>You can close this tab.</p></body></html>")
    } else if code.is_some() {
        "<html><body><h1>Symload Polar auth OK</h1><p>You can close this tab and return to the terminal.</p></body></html>"
            .to_string()
    } else {
        "<html><body><h1>Missing code</h1><p>No authorization code in redirect.</p></body></html>".to_string()
    };
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());

    if let Some(err) = error {
        return Err(SymError::UnsupportedFormat(format!("Polar OAuth error: {err}")));
    }
    let code = code.ok_or_else(|| SymError::UnsupportedFormat("OAuth redirect missing code parameter".into()))?;
    if let Some(st) = state {
        if st != expected_state {
            return Err(SymError::UnsupportedFormat(format!(
                "OAuth state mismatch (got {st}, expected {expected_state})"
            )));
        }
    }
    Ok(code)
}

fn parse_redirect_listen(redirect_uri: &str) -> Result<(String, u16, String), SymError> {
    // Expect http://host:port/path
    let rest = redirect_uri
        .strip_prefix("http://")
        .or_else(|| redirect_uri.strip_prefix("https://"))
        .ok_or_else(|| SymError::UnsupportedFormat("POLAR_REDIRECT_URI must be http://127.0.0.1:PORT/path".into()))?;
    let (hostport, path) = rest
        .split_once('/')
        .map(|(h, p)| (h, format!("/{p}")))
        .unwrap_or((rest, "/".into()));
    let (host, port) = if let Some((h, p)) = hostport.split_once(':') {
        let port: u16 = p
            .parse()
            .map_err(|_| SymError::UnsupportedFormat(format!("invalid port in redirect URI: {p}")))?;
        (h.to_string(), port)
    } else {
        (hostport.to_string(), 80)
    };
    Ok((host, port, path))
}

fn url_decode_minimal(s: &str) -> String {
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = &s[i + 1..i + 3];
                if let Ok(v) = u8::from_str_radix(hex, 16) {
                    out.push(v as char);
                    i += 3;
                } else {
                    out.push('%');
                    i += 1;
                }
            }
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

fn try_open_browser(url: &str) {
    let candidates = ["xdg-open", "open", "gio"];
    for cmd in candidates {
        if std::process::Command::new(cmd)
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .is_ok()
        {
            eprintln!("polar: launched browser via {cmd}");
            return;
        }
    }
    eprintln!("polar: could not auto-open browser — paste the URL manually");
}

fn now_iso_approx() -> String {
    use std::time::{
        SystemTime,
        UNIX_EPOCH,
    };
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

/// List recent exercises (AccessLink non-transactional; typically last ~30 days).
pub fn list_exercises(access_token: &str) -> Result<Vec<PolarExerciseSummary>, SymError> {
    let resp = ureq::get(&format!("{API_BASE}/exercises"))
        .set("Authorization", &format!("Bearer {access_token}"))
        .set("Accept", "application/json")
        .timeout(Duration::from_secs(60))
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, resp) => {
                let t = resp.into_string().unwrap_or_default();
                SymError::UnsupportedFormat(format!("list exercises HTTP {code}: {t}"))
            }
            other => SymError::UnsupportedFormat(format!("list exercises: {other}")),
        })?;

    let text = resp
        .into_string()
        .map_err(|e| SymError::UnsupportedFormat(format!("list body: {e}")))?;
    parse_exercise_list_json(&text)
}

/// Parse list-exercises JSON (array of objects with `id`).
pub fn parse_exercise_list_json(text: &str) -> Result<Vec<PolarExerciseSummary>, SymError> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| SymError::UnsupportedFormat(format!("exercises JSON: {e}")))?;
    let arr = v
        .as_array()
        .ok_or_else(|| SymError::UnsupportedFormat("exercises response is not a JSON array".into()))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let id = item
            .get("id")
            .and_then(|x| {
                x.as_str().map(|s| s.to_string()).or_else(|| {
                    // some payloads use numeric id
                    x.as_i64().map(|n| n.to_string())
                })
            })
            .ok_or_else(|| SymError::UnsupportedFormat("exercise missing id".into()))?;
        out.push(PolarExerciseSummary {
            id,
            start_time: item.get("start_time").and_then(|x| x.as_str()).map(|s| s.to_string()),
            upload_time: item.get("upload_time").and_then(|x| x.as_str()).map(|s| s.to_string()),
            sport: item.get("sport").and_then(|x| x.as_str()).map(|s| s.to_string()),
            device: item.get("device").and_then(|x| x.as_str()).map(|s| s.to_string()),
        });
    }
    Ok(out)
}

/// Download FIT bytes for one exercise id.
pub fn download_exercise_fit(access_token: &str, exercise_id: &str) -> Result<Vec<u8>, SymError> {
    let url = format!("{API_BASE}/exercises/{exercise_id}/fit");
    let resp = ureq::get(&url)
        .set("Authorization", &format!("Bearer {access_token}"))
        .set("Accept", "application/octet-stream, application/fit, */*")
        .timeout(Duration::from_secs(120))
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(code, resp) => {
                let t = resp.into_string().unwrap_or_default();
                SymError::UnsupportedFormat(format!("FIT download {exercise_id} HTTP {code}: {t}"))
            }
            other => SymError::UnsupportedFormat(format!("FIT download {exercise_id}: {other}")),
        })?;

    let mut bytes = Vec::new();
    resp.into_reader().read_to_end(&mut bytes).map_err(SymError::Io)?;
    if bytes.len() < 14 {
        return Err(SymError::UnsupportedFormat(format!(
            "FIT for {exercise_id} too small ({} bytes)",
            bytes.len()
        )));
    }
    Ok(bytes)
}

/// Stable filename for a Polar exercise FIT under `raw/polar/`.
///
/// Pattern: `polar_{exerciseId}.fit` — ingest parses `external_id` from this name.
pub fn polar_fit_filename(exercise_id: &str) -> String {
    let safe: String = exercise_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("polar_{safe}.fit")
}

/// Extract Polar exercise id from a landed filename (`polar_{id}.fit`).
pub fn external_id_from_polar_filename(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let stem = name.strip_suffix(".fit").or_else(|| name.strip_suffix(".FIT"))?;
    let id = stem.strip_prefix("polar_")?;
    if id.is_empty() { None } else { Some(id.to_string()) }
}

/// List exercises and download missing FITs into `target_dir` (default `raw/polar`).
pub fn fetch_exercise_fits(access_token: &str, target_dir: &Path, dry_run: bool) -> Result<PolarFetchReport, SymError> {
    fs::create_dir_all(target_dir).map_err(SymError::Io)?;
    let exercises = list_exercises(access_token)?;
    let mut report = PolarFetchReport {
        listed: exercises.len(),
        ..Default::default()
    };
    eprintln!(
        "polar: {} exercise(s) listed → {}",
        exercises.len(),
        target_dir.display()
    );

    for (i, ex) in exercises.iter().enumerate() {
        let n = i + 1;
        let fname = polar_fit_filename(&ex.id);
        let out = target_dir.join(&fname);
        if out.exists() {
            report.skipped_existing += 1;
            if n == 1 || n == exercises.len() || n % 10 == 0 {
                eprintln!(
                    "polar: progress {n}/{}  (new={} skip={} fail={})",
                    exercises.len(),
                    report.saved.len(),
                    report.skipped_existing,
                    report.failed
                );
            }
            continue;
        }
        if dry_run {
            eprintln!(
                "  [dry-run] would fetch {}  start={:?} sport={:?}",
                fname, ex.start_time, ex.sport
            );
            continue;
        }
        match download_exercise_fit(access_token, &ex.id) {
            Ok(bytes) => {
                let mut f = File::create(&out).map_err(SymError::Io)?;
                f.write_all(&bytes).map_err(SymError::Io)?;
                eprintln!(
                    "  + {}  ({} bytes)  start={:?}  {:?}",
                    fname,
                    bytes.len(),
                    ex.start_time,
                    ex.sport
                );
                report.saved.push(out);
            }
            Err(e) => {
                report.failed += 1;
                eprintln!("  ! {}: {e}", ex.id);
            }
        }
    }

    eprintln!(
        "polar: done — new={} skip_existing={} failed={} of {}",
        report.saved.len(),
        report.skipped_existing,
        report.failed,
        report.listed
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_and_external_id_roundtrip() {
        let f = polar_fit_filename("2AC312F");
        assert_eq!(f, "polar_2AC312F.fit");
        let id = external_id_from_polar_filename(Path::new(&f));
        assert_eq!(id.as_deref(), Some("2AC312F"));
    }

    #[test]
    fn parse_exercise_list_sample() {
        let json = r#"[
          {"id":"2AC312F","start_time":"2008-10-13T10:40:02","sport":"CYCLING","device":"Polar M400"},
          {"id":99,"start_time":"2020-01-01T00:00:00"}
        ]"#;
        let list = parse_exercise_list_json(json).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "2AC312F");
        assert_eq!(list[1].id, "99");
        assert_eq!(list[0].sport.as_deref(), Some("CYCLING"));
    }

    #[test]
    fn authorization_url_contains_client() {
        let c = PolarCredentials::new("abc-client", "secret", None, None);
        let u = c.authorization_url("st1");
        assert!(u.contains("client_id=abc-client"));
        assert!(u.contains("state=st1"));
        assert!(u.contains("response_type=code"));
    }
}
