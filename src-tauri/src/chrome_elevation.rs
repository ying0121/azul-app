//! Chrome v20 key access via a short-lived hidden elevated helper.
//! The main GUI app never relaunches as Administrator.

use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::chrome_abe;
use crate::chrome_ielevator;

pub fn is_key_extractor_mode() -> bool {
  #[cfg(target_os = "windows")]
  {
    return is_process_elevated() && elevation_request_pending();
  }
  #[cfg(not(target_os = "windows"))]
  {
    false
  }
}

/// Hidden elevated entry point: extract v20 key, cache it, exit (no GUI).
#[cfg(target_os = "windows")]
pub fn run_key_extractor() {
    let path = match chrome_abe::chrome_local_state_path() {
        Some(path) => path,
        None => std::process::exit(1),
    };

    let key = if chrome_ielevator::is_running_from_chrome_dir() {
        chrome_ielevator::extract_via_ielevator(&path)
    } else {
        chrome_abe::extract_app_bound_master_key(&path).or_else(|| {
            chrome_ielevator::spawn_chrome_path_helper_and_wait(
                &path,
                wait_for_cache,
                |p| load_v20_key_cache(p).is_some(),
            )
            .then(|| load_v20_key_cache(&path))
            .flatten()
        })
    };

    let key = match key {
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

/// At startup: if Chrome v20 is present, request UAC on every launch.
/// Returns true when elevation succeeded or is not required.
#[cfg(target_os = "windows")]
pub fn ensure_chrome_v20_elevation() -> bool {
    if is_key_extractor_mode() {
        return true;
    }

    if !chrome_abe::chrome_uses_v20() {
        return true;
    }

    let Some(local_state_path) = chrome_abe::chrome_local_state_path() else {
        return true;
    };

    if spawn_elevated_key_extractor_and_wait(&local_state_path) {
        mark_elevation_granted();
        return true;
    }

    ELEVATION_FAILURE.store(
        2,
        std::sync::atomic::Ordering::Relaxed,
    );
    false
}

#[cfg(not(target_os = "windows"))]
pub fn ensure_chrome_v20_elevation() -> bool {
    true
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
enum ElevationFailure {
    UacDeclined,
    KeyExtractionFailed,
}

#[cfg(target_os = "windows")]
static ELEVATION_FAILURE: std::sync::atomic::AtomicU8 =
    std::sync::atomic::AtomicU8::new(0);

pub fn show_elevation_failed_message() {
    #[cfg(target_os = "windows")]
    {
        let reason = match ELEVATION_FAILURE.load(std::sync::atomic::Ordering::Relaxed) {
            1 => ElevationFailure::UacDeclined,
            _ => ElevationFailure::KeyExtractionFailed,
        };
        show_elevation_failure_message(reason);
    }
}

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
    Ok(app_data_dir()?.join("cache"))
}

/// Remove cached v20 key on launch so elevation runs fresh each session.
pub fn clear_v20_key_cache() {
    if let Ok(path) = v20_key_cache_path() {
        let _ = std::fs::remove_file(path);
    }
    if let Ok(dir) = app_data_dir() {
        let _ = std::fs::remove_file(dir.join("chrome-v20-key.cache"));
    }
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
const ELEVATION_MUTEX_NAME: &str = "Local\\com.dailyhuddle.chrome-key-elevation";

#[cfg(target_os = "windows")]
struct ElevationMutexGuard(windows::Win32::Foundation::HANDLE);

#[cfg(target_os = "windows")]
impl ElevationMutexGuard {
    fn acquire() -> Option<Self> {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::CreateMutexW;

        let name: Vec<u16> = ELEVATION_MUTEX_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let handle = CreateMutexW(None, true, PCWSTR(name.as_ptr())).ok()?;
            if handle.is_invalid() {
                return None;
            }
            if windows::Win32::Foundation::GetLastError()
                == windows::Win32::Foundation::ERROR_ALREADY_EXISTS
            {
                let _ = CloseHandle(handle);
                return None;
            }
            Some(Self(handle))
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for ElevationMutexGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(target_os = "windows")]
fn elevation_request_pending() -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_ACCESS_RIGHTS};

    let name: Vec<u16> = ELEVATION_MUTEX_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let Ok(handle) = OpenMutexW(
            SYNCHRONIZATION_ACCESS_RIGHTS(0x0010_0000),
            false,
            PCWSTR(name.as_ptr()),
        ) else {
            return false;
        };
        let pending = !handle.is_invalid();
        let _ = CloseHandle(handle);
        pending
    }
}

#[cfg(target_os = "windows")]
fn is_process_elevated() -> bool {
    use std::mem::size_of;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut returned = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
        .is_ok();
        let _ = windows::Win32::Foundation::CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

#[cfg(target_os = "windows")]
fn wait_for_cache(local_state_path: &Path, attempts: u32) {
    for _ in 0..attempts {
        if load_v20_key_cache(local_state_path).is_some() {
            return;
        }
        thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(target_os = "windows")]
fn spawn_elevated_key_extractor_and_wait(local_state_path: &Path) -> bool {
    let Some(guard) = ElevationMutexGuard::acquire() else {
        wait_for_cache(local_state_path, 120);
        return load_v20_key_cache(local_state_path).is_some();
    };

    if !run_elevated_extraction(guard) {
        return false;
    }

    wait_for_cache(local_state_path, 20);
    load_v20_key_cache(local_state_path).is_some()
}

#[cfg(target_os = "windows")]
fn run_elevated_extraction(_guard: ElevationMutexGuard) -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };

    let exe_wide: Vec<u16> = OsStr::new(&exe)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: windows::Win32::UI::Shell::SEE_MASK_NOCLOSEPROCESS,
        lpVerb: windows::core::w!("runas"),
        lpFile: PCWSTR(exe_wide.as_ptr()),
        lpParameters: PCWSTR::null(),
        nShow: SW_HIDE.0 as i32,
        ..Default::default()
    };

    unsafe {
        if ShellExecuteExW(&mut info).is_err() {
            ELEVATION_FAILURE.store(1, std::sync::atomic::Ordering::Relaxed);
            return false;
        }

        if info.hProcess.is_invalid() {
            ELEVATION_FAILURE.store(1, std::sync::atomic::Ordering::Relaxed);
            return false;
        }

        let _ = WaitForSingleObject(info.hProcess, 120_000);
        let mut exit_code = 1u32;
        let _ = GetExitCodeProcess(info.hProcess, &mut exit_code);
        let _ = CloseHandle(info.hProcess);

        exit_code == 0
    }
}

#[cfg(target_os = "windows")]
fn dpapi_protect(data: &[u8]) -> Result<Vec<u8>, String> {
    crate::win_dpapi::protect(data)
}

#[cfg(target_os = "windows")]
fn dpapi_unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    crate::win_dpapi::unprotect(data)
}

#[cfg(target_os = "windows")]
fn show_elevation_failure_message(reason: ElevationFailure) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let text = match reason {
        ElevationFailure::UacDeclined => {
            "Administrator permission is required to read Chrome data.\n\n\
             Please click Yes on the UAC prompt and try again."
        }
        ElevationFailure::KeyExtractionFailed => {
            "Administrator permission was granted, but Chrome data access failed.\n\n\
             This can happen on Windows 11 when Chrome's security settings block access. \
             Make sure Google Chrome is installed and try again. \
             If the problem continues, contact support."
        }
    };

    let text: Vec<u16> = OsStr::new(text)
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
