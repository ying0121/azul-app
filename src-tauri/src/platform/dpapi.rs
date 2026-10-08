#[cfg(target_os = "windows")]
pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
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
pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
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

#[cfg(not(target_os = "windows"))]
pub fn protect(_data: &[u8]) -> Result<Vec<u8>, String> {
    Err("Not Allowed".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn unprotect(_data: &[u8]) -> Result<Vec<u8>, String> {
    Err("Not Allowed".to_string())
}
