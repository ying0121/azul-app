//! Structured extraction of all known Chrome stored data types for a profile.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::chrome::{
    build_analyze_meta, chrome_user_data_dir, copy_sqlite_to_temp, decrypt_chrome_secret,
    is_sqlite_file_in_use_error, read_cookies, read_passwords, read_session_restore,
    read_session_snapshots, resolve_chrome_keys, resolve_cookies_path, resolve_profile_names,
    sanitize_label, ChromeAnalyzeMeta, ChromeCookieEntry, ChromeKeys, ChromePasswordEntry,
    ChromeSessionEntry,
};

const HISTORY_LIMIT_PER_PROFILE: usize = 2_000;
const AUTOFILL_LIMIT_PER_PROFILE: usize = 1_000;
const SITE_SETTINGS_LIMIT_PER_PROFILE: usize = 500;

#[derive(Debug, Serialize)]
pub struct DataSection<T> {
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub skipped: bool,
    pub entries: Vec<T>,
}

impl<T> DataSection<T> {
    fn ok(entries: Vec<T>) -> Self {
        let count = entries.len();
        Self {
            count,
            error: None,
            skipped: false,
            entries,
        }
    }

    fn err(message: impl Into<String>) -> Self {
        Self {
            count: 0,
            error: Some(message.into()),
            skipped: false,
            entries: Vec::new(),
        }
    }

    fn skipped(message: impl Into<String>) -> Self {
        Self {
            count: 0,
            error: Some(message.into()),
            skipped: true,
            entries: Vec::new(),
        }
    }

    fn missing() -> Self {
        Self::err("Not found")
    }

    fn empty() -> Self {
        Self {
            count: 0,
            error: None,
            skipped: false,
            entries: Vec::new(),
        }
    }

    fn merge(sections: impl IntoIterator<Item = DataSection<T>>) -> Self {
        let mut entries = Vec::new();
        let mut errors = Vec::new();
        let mut skipped_notes = Vec::new();
        let mut skipped_sections = 0;

        for section in sections {
            if section.skipped {
                skipped_sections += 1;
                if let Some(error) = section.error {
                    skipped_notes.push(error);
                }
                continue;
            }
            if let Some(error) = section.error {
                if error != "Not found" {
                    errors.push(error);
                }
            }
            entries.extend(section.entries);
        }

        let all_skipped =
            skipped_sections > 0 && entries.is_empty() && errors.is_empty();

        Self {
            count: entries.len(),
            skipped: all_skipped,
            error: if all_skipped {
                Some(skipped_notes.join("; "))
            } else if !errors.is_empty() {
                Some(errors.join("; "))
            } else if skipped_notes.is_empty() {
                None
            } else {
                Some(skipped_notes.join("; "))
            },
            entries,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ChromeHistoryEntry {
    pub profile: String,
    pub url: String,
    pub title: String,
    pub visit_count: i64,
    pub last_visit_time: Option<i64>,
    pub visit_time: Option<i64>,
    pub transition: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChromeBookmarkEntry {
    pub profile: String,
    pub name: String,
    pub url: String,
    pub folder_path: String,
    pub date_added: Option<i64>,
    pub date_modified: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChromeAutofillEntry {
    pub profile: String,
    pub name: String,
    pub value: String,
    pub date_created: Option<i64>,
    pub date_last_used: Option<i64>,
    pub count: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChromeAddressEntry {
    pub profile: String,
    pub guid: String,
    pub full_name: String,
    pub company_name: String,
    pub street_address: String,
    pub city: String,
    pub state: String,
    pub zipcode: String,
    pub country_code: String,
    pub phone_number: String,
    pub email: String,
    pub date_modified: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChromeCreditCardEntry {
    pub profile: String,
    pub name_on_card: String,
    pub expiration_month: Option<i64>,
    pub expiration_year: Option<i64>,
    pub card_number: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub card_decrypt_failed: bool,
    pub date_modified: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChromeExtensionEntry {
    pub profile: String,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub install_path: String,
    pub homepage_url: String,
}

#[derive(Debug, Serialize)]
pub struct ChromeSiteSettingEntry {
    pub profile: String,
    pub setting: String,
    pub origin: String,
    pub value: Value,
}

#[derive(Debug, Serialize)]
pub struct ChromeTransportSecurityEntry {
    pub profile: String,
    pub host: String,
    pub sts_include_subdomains: bool,
    pub created: Option<i64>,
    pub expiry: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChromeStoredDataResult {
    pub meta: ChromeAnalyzeMeta,
    pub profile: String,
    pub passwords: DataSection<ChromePasswordEntry>,
    pub cookies: DataSection<ChromeCookieEntry>,
    pub sessions: DataSection<ChromeSessionEntry>,
    pub session_restore: DataSection<ChromeSessionEntry>,
    pub history: DataSection<ChromeHistoryEntry>,
    pub bookmarks: DataSection<ChromeBookmarkEntry>,
    pub autofill: DataSection<ChromeAutofillEntry>,
    pub addresses: DataSection<ChromeAddressEntry>,
    pub credit_cards: DataSection<ChromeCreditCardEntry>,
    pub extensions: DataSection<ChromeExtensionEntry>,
    pub site_settings: DataSection<ChromeSiteSettingEntry>,
    pub transport_security: DataSection<ChromeTransportSecurityEntry>,
    pub account_passwords: DataSection<ChromePasswordEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoredDataCategory {
    Passwords,
    AccountPasswords,
    Cookies,
    Sessions,
    SessionRestore,
    History,
    Bookmarks,
    Autofill,
    Addresses,
    CreditCards,
    Extensions,
    SiteSettings,
    TransportSecurity,
}

pub fn parse_stored_data_category(value: &str) -> Result<StoredDataCategory, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "passwords" => Ok(StoredDataCategory::Passwords),
        "account_passwords" => Ok(StoredDataCategory::AccountPasswords),
        "cookies" => Ok(StoredDataCategory::Cookies),
        "sessions" => Ok(StoredDataCategory::Sessions),
        "session_restore" => Ok(StoredDataCategory::SessionRestore),
        "history" => Ok(StoredDataCategory::History),
        "bookmarks" => Ok(StoredDataCategory::Bookmarks),
        "autofill" => Ok(StoredDataCategory::Autofill),
        "addresses" => Ok(StoredDataCategory::Addresses),
        "credit_cards" => Ok(StoredDataCategory::CreditCards),
        "extensions" => Ok(StoredDataCategory::Extensions),
        "site_settings" => Ok(StoredDataCategory::SiteSettings),
        "transport_security" => Ok(StoredDataCategory::TransportSecurity),
        other => Err(format!("Unknown category: {other}")),
    }
}

fn section_for<T>(
    requested: StoredDataCategory,
    target: StoredDataCategory,
    sections: Vec<DataSection<T>>,
) -> DataSection<T> {
    if requested == target {
        DataSection::merge(sections)
    } else {
        DataSection::empty()
    }
}

pub fn analyze_stored_data(profile: &str, category: &str) -> Result<ChromeStoredDataResult, String> {
    crate::chrome_analysis::begin_chrome_analysis();
    let category = parse_stored_data_category(category)?;
    let user_data = chrome_user_data_dir()?;
    let profile_names = resolve_profile_names(&user_data, profile)?;
    let meta = build_analyze_meta(&user_data)?;
    let keys = resolve_chrome_keys(&user_data)?;

    let mut passwords = Vec::new();
    let mut cookies = Vec::new();
    let mut sessions = Vec::new();
    let mut session_restore = Vec::new();
    let mut history = Vec::new();
    let mut bookmarks = Vec::new();
    let mut autofill = Vec::new();
    let mut addresses = Vec::new();
    let mut credit_cards = Vec::new();
    let mut extensions = Vec::new();
    let mut site_settings = Vec::new();
    let mut transport_security = Vec::new();
    let mut account_passwords = Vec::new();

    for profile_name in &profile_names {
        crate::chrome_analysis::check_chrome_analysis_cancelled()?;
        let profile_dir = user_data.join(profile_name);

        match category {
            StoredDataCategory::Passwords => {
                passwords.push(collect_passwords(
                    &profile_dir,
                    profile_name,
                    &keys,
                    "Login Data",
                ));
            }
            StoredDataCategory::AccountPasswords => {
                account_passwords.push(collect_passwords(
                    &profile_dir,
                    profile_name,
                    &keys,
                    "Login Data For Account",
                ));
            }
            StoredDataCategory::Cookies => {
                cookies.push(collect_cookies(&profile_dir, profile_name, &keys));
            }
            StoredDataCategory::Sessions => {
                sessions.push(collect_sessions(&profile_dir, profile_name));
            }
            StoredDataCategory::SessionRestore => {
                session_restore.push(collect_session_restore(&profile_dir, profile_name));
            }
            StoredDataCategory::History => {
                history.push(collect_history(&profile_dir, profile_name));
            }
            StoredDataCategory::Bookmarks => {
                bookmarks.push(collect_bookmarks(&profile_dir, profile_name));
            }
            StoredDataCategory::Autofill => {
                autofill.push(with_profile_db(&profile_dir, "Web Data", "autofill", |conn| {
                    read_autofill_from_db(conn, profile_name)
                }));
            }
            StoredDataCategory::Addresses => {
                addresses.push(with_profile_db(&profile_dir, "Web Data", "addresses", |conn| {
                    read_addresses_from_db(conn, profile_name, &keys)
                }));
            }
            StoredDataCategory::CreditCards => {
                credit_cards.push(with_profile_db(&profile_dir, "Web Data", "credit-cards", |conn| {
                    read_credit_cards_from_db(conn, profile_name, &keys)
                }));
            }
            StoredDataCategory::Extensions => {
                let (extensions_section, _) =
                    collect_preferences_sections(&profile_dir, profile_name);
                extensions.push(extensions_section);
            }
            StoredDataCategory::SiteSettings => {
                let (_, site_settings_section) =
                    collect_preferences_sections(&profile_dir, profile_name);
                site_settings.push(site_settings_section);
            }
            StoredDataCategory::TransportSecurity => {
                transport_security.push(collect_transport_security(&profile_dir, profile_name));
            }
        }
    }

    let result_profile = if profile_names.len() == 1 {
        profile_names[0].clone()
    } else {
        "all".to_string()
    };

    Ok(ChromeStoredDataResult {
        meta,
        profile: result_profile,
        passwords: section_for(category, StoredDataCategory::Passwords, passwords),
        cookies: section_for(category, StoredDataCategory::Cookies, cookies),
        sessions: section_for(category, StoredDataCategory::Sessions, sessions),
        session_restore: section_for(
            category,
            StoredDataCategory::SessionRestore,
            session_restore,
        ),
        history: section_for(category, StoredDataCategory::History, history),
        bookmarks: section_for(category, StoredDataCategory::Bookmarks, bookmarks),
        autofill: section_for(category, StoredDataCategory::Autofill, autofill),
        addresses: section_for(category, StoredDataCategory::Addresses, addresses),
        credit_cards: section_for(category, StoredDataCategory::CreditCards, credit_cards),
        extensions: section_for(category, StoredDataCategory::Extensions, extensions),
        site_settings: section_for(category, StoredDataCategory::SiteSettings, site_settings),
        transport_security: section_for(
            category,
            StoredDataCategory::TransportSecurity,
            transport_security,
        ),
        account_passwords: section_for(
            category,
            StoredDataCategory::AccountPasswords,
            account_passwords,
        ),
    })
}

fn collect_passwords(
    profile_dir: &Path,
    profile: &str,
    keys: &ChromeKeys,
    db_name: &str,
) -> DataSection<ChromePasswordEntry> {
    let db_path = profile_dir.join(db_name);
    if !db_path.is_file() {
        return DataSection::missing();
    }

    let label = format!("{}-{}", sanitize_label(db_name), sanitize_label(profile));
    match copy_sqlite_to_temp(&db_path, &label) {
        Ok(temp) => {
            let result = read_passwords(&temp, keys, profile);
            let _ = std::fs::remove_file(&temp);
            match result {
                Ok(entries) => DataSection::ok(entries),
                Err(error) => DataSection::err(error),
            }
        }
        Err(error) => DataSection::err(error),
    }
}

fn collect_cookies(
    profile_dir: &Path,
    profile: &str,
    keys: &ChromeKeys,
) -> DataSection<ChromeCookieEntry> {
    let cookies_path = match resolve_cookies_path(profile_dir) {
        Ok(path) => path,
        Err(_) => return DataSection::missing(),
    };

    let label = format!("cookies-{}", sanitize_label(profile));
    match copy_sqlite_to_temp(&cookies_path, &label) {
        Ok(temp) => {
            let result = read_cookies(&temp, keys, profile);
            let _ = std::fs::remove_file(&temp);
            match result {
                Ok(entries) => DataSection::ok(entries),
                Err(error) => DataSection::err(error),
            }
        }
        Err(error) => {
            if is_sqlite_file_in_use_error(&error) {
                DataSection::skipped(format!("Skipped: cookies file in use ({profile})"))
            } else {
                DataSection::err(error)
            }
        }
    }
}

fn collect_sessions(profile_dir: &Path, profile: &str) -> DataSection<ChromeSessionEntry> {
    match read_session_snapshots(profile_dir, profile) {
        Ok(entries) => DataSection::ok(entries),
        Err(error) => DataSection::err(error),
    }
}

fn collect_session_restore(profile_dir: &Path, profile: &str) -> DataSection<ChromeSessionEntry> {
    match read_session_restore(profile_dir, profile) {
        Ok(entries) => DataSection::ok(entries),
        Err(error) => DataSection::err(error),
    }
}

fn collect_history(profile_dir: &Path, profile: &str) -> DataSection<ChromeHistoryEntry> {
    with_profile_db(profile_dir, "History", "history", |conn| {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT u.url, u.title, u.visit_count, u.last_visit_time, v.visit_time, v.transition \
                 FROM visits v \
                 JOIN urls u ON v.url = u.id \
                 ORDER BY v.visit_time DESC \
                 LIMIT {}",
                HISTORY_LIMIT_PER_PROFILE
            ))
            .map_err(|e| format!("Error was occurred: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ChromeHistoryEntry {
                    profile: profile.to_string(),
                    url: row.get(0)?,
                    title: row.get(1)?,
                    visit_count: row.get(2)?,
                    last_visit_time: row.get(3)?,
                    visit_time: row.get(4)?,
                    transition: row.get(5)?,
                })
            })
            .map_err(|e| format!("Error was occurred: {e}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Error was occurred: {e}"))
    })
}

fn collect_bookmarks(profile_dir: &Path, profile: &str) -> DataSection<ChromeBookmarkEntry> {
    let path = profile_dir.join("Bookmarks");
    if !path.is_file() {
        return DataSection::missing();
    }

    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) => return DataSection::err(format!("Error was occurred: {error}")),
    };
    let json: Value = match serde_json::from_str(&raw) {
        Ok(json) => json,
        Err(error) => return DataSection::err(format!("Invalid JSON: {error}")),
    };

    let mut entries = Vec::new();
    if let Some(roots) = json.get("roots").and_then(Value::as_object) {
        for (root_name, root) in roots {
            flatten_bookmarks(profile, root, root_name, &mut entries);
        }
    }

    DataSection::ok(entries)
}

fn flatten_bookmarks(
    profile: &str,
    node: &Value,
    folder_path: &str,
    out: &mut Vec<ChromeBookmarkEntry>,
) {
    if node.get("type").and_then(Value::as_str) == Some("url") {
        out.push(ChromeBookmarkEntry {
            profile: profile.to_string(),
            name: node
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            url: node
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            folder_path: folder_path.to_string(),
            date_added: node.get("date_added").and_then(Value::as_str).and_then(parse_chrome_time),
            date_modified: node
                .get("date_modified")
                .and_then(Value::as_str)
                .and_then(parse_chrome_time),
        });
    }

    if let Some(children) = node.get("children").and_then(Value::as_array) {
        let child_folder = if node.get("type").and_then(Value::as_str) == Some("folder") {
            let name = node
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("folder");
            if folder_path.is_empty() {
                name.to_string()
            } else {
                format!("{folder_path}/{name}")
            }
        } else {
            folder_path.to_string()
        };

        for child in children {
            flatten_bookmarks(profile, child, &child_folder, out);
        }
    }
}

fn parse_chrome_time(value: &str) -> Option<i64> {
    value.parse::<i64>().ok()
}

fn read_autofill_from_db(
    conn: &rusqlite::Connection,
    profile: &str,
) -> Result<Vec<ChromeAutofillEntry>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name, value, date_created, date_last_used, count \
             FROM autofill ORDER BY date_last_used DESC \
             LIMIT {}",
            AUTOFILL_LIMIT_PER_PROFILE
        ))
        .map_err(|e| format!("Error was occurred: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ChromeAutofillEntry {
                profile: profile.to_string(),
                name: row.get(0)?,
                value: row.get(1)?,
                date_created: row.get(2)?,
                date_last_used: row.get(3)?,
                count: row.get(4)?,
            })
        })
        .map_err(|e| format!("Error was occurred: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Error was occurred: {e}"))
}

fn read_credit_cards_from_db(
    conn: &rusqlite::Connection,
    profile: &str,
    keys: &ChromeKeys,
) -> Result<Vec<ChromeCreditCardEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT name_on_card, expiration_month, expiration_year, card_number_encrypted, date_modified \
             FROM credit_cards ORDER BY date_modified DESC",
        )
        .map_err(|e| format!("Error was occurred: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let name_on_card: String = row.get(0)?;
            let expiration_month: Option<i64> = row.get(1)?;
            let expiration_year: Option<i64> = row.get(2)?;
            let encrypted: Vec<u8> = row.get(3)?;
            let date_modified: Option<i64> = row.get(4)?;
            let (card_number, card_decrypt_failed) = match decrypt_chrome_secret(keys, &encrypted) {
                Ok(value) => (value, false),
                Err(_) => (String::new(), !encrypted.is_empty()),
            };
            Ok(ChromeCreditCardEntry {
                profile: profile.to_string(),
                name_on_card,
                expiration_month,
                expiration_year,
                card_number,
                card_decrypt_failed,
                date_modified,
            })
        })
        .map_err(|e| format!("Error was occurred: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Error was occurred: {e}"))
}

fn collect_preferences_sections(
    profile_dir: &Path,
    profile: &str,
) -> (
    DataSection<ChromeExtensionEntry>,
    DataSection<ChromeSiteSettingEntry>,
) {
    let preferences_json = read_optional_preferences_json(&profile_dir.join("Preferences"));
    let secure_json = read_optional_preferences_json(&profile_dir.join("Secure Preferences"));

    if preferences_json.is_none() && secure_json.is_none() {
        return (DataSection::missing(), DataSection::missing());
    }

    let settings = merged_extension_settings(preferences_json.as_ref(), secure_json.as_ref());
    let extensions = if settings.is_empty() {
        collect_extensions_from_disk(profile_dir, profile)
    } else {
        read_extensions_from_settings(&settings, profile_dir, profile)
    };

    let site_settings_source = preferences_json
        .as_ref()
        .or(secure_json.as_ref())
        .map(|value| value as &Value)
        .unwrap_or(&Value::Null);
    let site_settings = read_site_settings_from_preferences(site_settings_source, profile);

    (extensions, site_settings)
}

fn read_optional_preferences_json(path: &Path) -> Option<Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn merged_extension_settings(
    preferences: Option<&Value>,
    secure_preferences: Option<&Value>,
) -> serde_json::Map<String, Value> {
    let mut settings = serde_json::Map::new();
    for source in [preferences, secure_preferences] {
        let Some(json) = source else { continue };
        let Some(obj) = json
            .pointer("/extensions/settings")
            .and_then(Value::as_object)
        else {
            continue;
        };
        for (id, setting) in obj {
            settings.insert(id.clone(), setting.clone());
        }
    }
    settings
}

fn read_extensions_from_settings(
    settings: &serde_json::Map<String, Value>,
    profile_dir: &Path,
    profile: &str,
) -> DataSection<ChromeExtensionEntry> {
    let mut entries = Vec::new();
    for (id, setting) in settings {
        entries.push(extension_entry_from_setting(profile_dir, profile, id, setting));
    }

    entries.sort_by(|left, right| left.name.cmp(&right.name));
    DataSection::ok(entries)
}

fn extension_entry_from_setting(
    profile_dir: &Path,
    profile: &str,
    id: &str,
    setting: &Value,
) -> ChromeExtensionEntry {
    let manifest = setting.get("manifest").cloned().unwrap_or(Value::Null);
    let path = setting
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    ChromeExtensionEntry {
        profile: profile.to_string(),
        id: id.to_string(),
        name: manifest
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        version: manifest
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        description: manifest
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        enabled: is_extension_enabled(setting),
        install_path: resolve_extension_install_path(profile_dir, path),
        homepage_url: manifest
            .get("homepage_url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    }
}

fn is_extension_enabled(setting: &Value) -> bool {
    if let Some(state) = setting.get("state").and_then(Value::as_i64) {
        return state == 1;
    }

    setting
        .get("disable_reasons")
        .and_then(Value::as_array)
        .is_none_or(|reasons| reasons.is_empty())
}

fn resolve_extension_install_path(profile_dir: &Path, path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }

    let path_buf = PathBuf::from(path);
    if path_buf.is_absolute() {
        return path_buf.display().to_string();
    }

    profile_dir
        .join("Extensions")
        .join(path)
        .display()
        .to_string()
}

fn collect_extensions_from_disk(
    profile_dir: &Path,
    profile: &str,
) -> DataSection<ChromeExtensionEntry> {
    let extensions_dir = profile_dir.join("Extensions");
    if !extensions_dir.is_dir() {
        return DataSection::missing();
    }

    let mut entries = Vec::new();
    let Ok(dir_entries) = std::fs::read_dir(&extensions_dir) else {
        return DataSection::err("Error was occurred: failed to read Extensions directory");
    };

    for entry in dir_entries.flatten() {
        let extension_id = entry.file_name().to_string_lossy().into_owned();
        let extension_dir = entry.path();
        if !extension_dir.is_dir() {
            continue;
        }

        let Some(manifest_path) = find_extension_manifest(&extension_dir) else {
            continue;
        };

        let manifest = match read_optional_preferences_json(&manifest_path) {
            Some(manifest) => manifest,
            None => continue,
        };

        entries.push(ChromeExtensionEntry {
            profile: profile.to_string(),
            id: extension_id.clone(),
            name: manifest
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(&extension_id)
                .to_string(),
            version: manifest
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            description: manifest
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            enabled: true,
            install_path: manifest_path
                .parent()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            homepage_url: manifest
                .get("homepage_url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        });
    }

    entries.sort_by(|left, right| left.name.cmp(&right.name));
    DataSection::ok(entries)
}

fn find_extension_manifest(extension_dir: &Path) -> Option<PathBuf> {
    let mut versions = Vec::new();
    let dir_entries = std::fs::read_dir(extension_dir).ok()?;
    for entry in dir_entries.flatten() {
        let version_dir = entry.path();
        if !version_dir.is_dir() {
            continue;
        }
        let manifest_path = version_dir.join("manifest.json");
        if manifest_path.is_file() {
            versions.push(manifest_path);
        }
    }

    versions.sort();
    versions.pop()
}

fn read_site_settings_from_preferences(
    json: &Value,
    profile: &str,
) -> DataSection<ChromeSiteSettingEntry> {
    let mut entries = Vec::new();
    if let Some(exceptions) = json
        .pointer("/profile/content_settings/exceptions")
        .and_then(Value::as_object)
    {
        'outer: for (setting, origins) in exceptions {
            if let Some(origin_map) = origins.as_object() {
                for (origin, value) in origin_map {
                    entries.push(ChromeSiteSettingEntry {
                        profile: profile.to_string(),
                        setting: setting.clone(),
                        origin: origin.clone(),
                        value: value.clone(),
                    });
                    if entries.len() >= SITE_SETTINGS_LIMIT_PER_PROFILE {
                        break 'outer;
                    }
                }
            }
        }
    }

    DataSection::ok(entries)
}

fn read_addresses_from_db(
    conn: &rusqlite::Connection,
    profile: &str,
    keys: &ChromeKeys,
) -> Result<Vec<ChromeAddressEntry>, String> {
    if table_exists(conn, "autofill_profiles") {
        return read_addresses_from_profiles_table(conn, profile);
    }

    if table_exists(conn, "autofill_ai_entities") && table_exists(conn, "autofill_ai_attributes")
    {
        return read_addresses_from_autofill_ai(conn, profile, keys);
    }

    Ok(Vec::new())
}

fn read_addresses_from_profiles_table(
    conn: &rusqlite::Connection,
    profile: &str,
) -> Result<Vec<ChromeAddressEntry>, String> {
    if table_exists(conn, "autofill_profile_names") {
        let mut stmt = conn
            .prepare(
                "SELECT p.guid, \
                 COALESCE(n.full_name, ''), COALESCE(p.company_name, ''), \
                 COALESCE(p.street_address, ''), COALESCE(p.city, ''), COALESCE(p.state, ''), \
                 COALESCE(p.zipcode, ''), COALESCE(p.country_code, ''), \
                 COALESCE((SELECT number FROM autofill_profile_phones WHERE guid = p.guid LIMIT 1), ''), \
                 COALESCE((SELECT email FROM autofill_profile_emails WHERE guid = p.guid LIMIT 1), ''), \
                 p.date_modified \
                 FROM autofill_profiles p \
                 LEFT JOIN autofill_profile_names n ON p.guid = n.guid \
                 ORDER BY p.date_modified DESC",
            )
            .map_err(|e| format!("Error was occurred: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ChromeAddressEntry {
                    profile: profile.to_string(),
                    guid: row.get(0)?,
                    full_name: row.get(1)?,
                    company_name: row.get(2)?,
                    street_address: row.get(3)?,
                    city: row.get(4)?,
                    state: row.get(5)?,
                    zipcode: row.get(6)?,
                    country_code: row.get(7)?,
                    phone_number: row.get(8)?,
                    email: row.get(9)?,
                    date_modified: row.get(10)?,
                })
            })
            .map_err(|e| format!("Error was occurred: {e}"))?;

        return rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Error was occurred: {e}"));
    }

    let mut stmt = conn
        .prepare(
            "SELECT guid, full_name, company_name, street_address, city, state, zipcode, \
             country_code, phone_number, email, date_modified \
             FROM autofill_profiles ORDER BY date_modified DESC",
        )
        .map_err(|e| format!("Error was occurred: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ChromeAddressEntry {
                profile: profile.to_string(),
                guid: row.get(0)?,
                full_name: row.get(1)?,
                company_name: row.get(2)?,
                street_address: row.get(3)?,
                city: row.get(4)?,
                state: row.get(5)?,
                zipcode: row.get(6)?,
                country_code: row.get(7)?,
                phone_number: row.get(8)?,
                email: row.get(9)?,
                date_modified: row.get(10)?,
            })
        })
        .map_err(|e| format!("Error was occurred: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Error was occurred: {e}"))
}

fn read_addresses_from_autofill_ai(
    conn: &rusqlite::Connection,
    profile: &str,
    keys: &ChromeKeys,
) -> Result<Vec<ChromeAddressEntry>, String> {
    let has_metadata = table_exists(conn, "autofill_ai_entities_metadata");
    let query = if has_metadata {
        "SELECT e.guid, e.entity_type, e.nickname, m.date_modified, \
         a.attribute_type, a.value_encrypted \
         FROM autofill_ai_entities e \
         JOIN autofill_ai_attributes a ON e.guid = a.entity_guid \
         LEFT JOIN autofill_ai_entities_metadata m ON e.guid = m.entity_guid \
         ORDER BY m.date_modified DESC"
    } else {
        "SELECT e.guid, e.entity_type, e.nickname, NULL, \
         a.attribute_type, a.value_encrypted \
         FROM autofill_ai_entities e \
         JOIN autofill_ai_attributes a ON e.guid = a.entity_guid \
         ORDER BY e.nickname"
    };

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("Error was occurred: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Vec<u8>>(5)?,
            ))
        })
        .map_err(|e| format!("Error was occurred: {e}"))?;

    let mut by_guid: std::collections::BTreeMap<String, ChromeAddressEntry> =
        std::collections::BTreeMap::new();

    for row in rows {
        let (guid, entity_type, nickname, date_modified, attribute_type, encrypted) =
            row.map_err(|e| format!("Error was occurred: {e}"))?;

        if !is_address_entity_type(&entity_type) {
            continue;
        }

        let value = decrypt_chrome_secret(keys, &encrypted).unwrap_or_default();
        let entry = by_guid.entry(guid.clone()).or_insert_with(|| ChromeAddressEntry {
            profile: profile.to_string(),
            guid,
            full_name: nickname,
            company_name: String::new(),
            street_address: String::new(),
            city: String::new(),
            state: String::new(),
            zipcode: String::new(),
            country_code: String::new(),
            phone_number: String::new(),
            email: String::new(),
            date_modified,
        });

        apply_autofill_ai_attribute(&attribute_type, &value, entry);
    }

    Ok(by_guid.into_values().collect())
}

fn is_address_entity_type(entity_type: &str) -> bool {
    let normalized = entity_type.to_ascii_lowercase();
    normalized.contains("address") || normalized.contains("profile")
}

fn apply_autofill_ai_attribute(attribute_type: &str, value: &str, entry: &mut ChromeAddressEntry) {
    if value.is_empty() {
        return;
    }

    let key = attribute_type.to_ascii_uppercase().replace('-', "_");
    if key.contains("COMPANY") {
        entry.company_name = value.to_string();
    } else if key.contains("STREET") || key.contains("ADDRESS_LINE") || key == "ADDRESS" {
        entry.street_address = value.to_string();
    } else if key.contains("CITY") || key.contains("LOCALITY") {
        entry.city = value.to_string();
    } else if key.contains("STATE") || key.contains("REGION") {
        entry.state = value.to_string();
    } else if key.contains("ZIP") || key.contains("POSTAL") {
        entry.zipcode = value.to_string();
    } else if key.contains("COUNTRY") {
        entry.country_code = value.to_string();
    } else if key.contains("PHONE") {
        entry.phone_number = value.to_string();
    } else if key.contains("EMAIL") {
        entry.email = value.to_string();
    } else if key.contains("NAME") {
        entry.full_name = value.to_string();
    }
}

fn table_exists(conn: &rusqlite::Connection, table_name: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table_name],
        |_| Ok(()),
    )
    .is_ok()
}

fn collect_transport_security(
    profile_dir: &Path,
    profile: &str,
) -> DataSection<ChromeTransportSecurityEntry> {
    let path = profile_dir.join("TransportSecurity");
    if !path.is_file() {
        return DataSection::missing();
    }

    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) => return DataSection::err(format!("Error was occurred: {error}")),
    };
    let json: Value = match serde_json::from_str(&raw) {
        Ok(json) => json,
        Err(error) => return DataSection::err(format!("Invalid JSON: {error}")),
    };

    let mut entries = Vec::new();
    if let Some(sts) = json.get("sts").and_then(Value::as_array) {
        for item in sts {
            entries.push(ChromeTransportSecurityEntry {
                profile: profile.to_string(),
                host: item
                    .get("host")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                sts_include_subdomains: item
                    .get("sts_include_subdomains")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                created: item.get("created").and_then(Value::as_i64),
                expiry: item.get("expiry").and_then(Value::as_i64),
            });
        }
    }

    DataSection::ok(entries)
}

fn with_profile_db<T, F>(
    profile_dir: &Path,
    db_name: &str,
    label: &str,
    read: F,
) -> DataSection<T>
where
    F: FnOnce(&rusqlite::Connection) -> Result<Vec<T>, String>,
{
    let db_path = profile_dir.join(db_name);
    if !db_path.is_file() {
        return DataSection::missing();
    }

    let temp_label = format!("{}-{}", sanitize_label(label), sanitize_label(db_name));
    let temp = match copy_sqlite_to_temp(&db_path, &temp_label) {
        Ok(temp) => temp,
        Err(error) => return DataSection::err(error),
    };

    let conn = match rusqlite::Connection::open_with_flags(
        &temp,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(conn) => conn,
        Err(error) => {
            let _ = std::fs::remove_file(&temp);
            return DataSection::err(format!("Error was occurred: {error}"));
        }
    };

    let result = read(&conn);
    let _ = std::fs::remove_file(&temp);

    match result {
        Ok(entries) => DataSection::ok(entries),
        Err(error) => DataSection::err(error),
    }
}

#[tauri::command]
pub fn chrome_analyze_stored_data(
    profile: String,
    category: String,
) -> Result<ChromeStoredDataResult, String> {
    let profile = crate::chrome::normalize_profile_param(&profile);
    analyze_stored_data(&profile, &category)
}
