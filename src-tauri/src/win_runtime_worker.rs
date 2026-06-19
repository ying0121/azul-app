//! Windows: run the real app from a copy under AppData so the installed `.exe`
//! is not locked and can be deleted in Explorer while the tray app keeps running.

#[cfg(windows)]
mod imp {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    const WORKER_ARG: &str = "--worker";
    const RUNTIME_DIR: &str = "com.dailyhuddle.desktop\\runtime";
    const RUNTIME_EXE: &str = "daily-huddle.exe";

    struct DeleteShareHandle(HANDLE);

    impl Drop for DeleteShareHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    pub fn relaunch_as_runtime_worker() -> bool {
        if is_worker_process() {
            return false;
        }

        if cfg!(debug_assertions) {
            return false;
        }

        let Ok(install_exe) = std::env::current_exe() else {
            return false;
        };

        if is_runtime_exe(&install_exe) {
            return false;
        }

        let Some(runtime_exe) = runtime_exe_path() else {
            return false;
        };

        if !stage_runtime_copy(&install_exe, &runtime_exe) {
            return false;
        }

        if !spawn_worker(&runtime_exe) {
            return false;
        }

        true
    }

    pub fn enable_delete_while_running() -> bool {
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        hold_delete_share_handle(&exe)
    }

    fn is_worker_process() -> bool {
        std::env::args().any(|arg| arg == WORKER_ARG)
    }

    fn is_runtime_exe(path: &Path) -> bool {
        let lossy = path.to_string_lossy();
        lossy.contains("com.dailyhuddle.desktop")
            && lossy.contains("runtime")
            && lossy.ends_with(RUNTIME_EXE)
    }

    fn runtime_exe_path() -> Option<PathBuf> {
        let local_app_data = std::env::var_os("LOCALAPPDATA")?;
        Some(
            PathBuf::from(local_app_data)
                .join(RUNTIME_DIR)
                .join(RUNTIME_EXE),
        )
    }

    fn stage_runtime_copy(source: &Path, dest: &Path) -> bool {
        if let Some(parent) = dest.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return false;
            }
        }

        if dest.exists() {
            let _ = std::fs::remove_file(dest);
        }

        std::fs::copy(source, dest).is_ok() || dest.exists()
    }

    fn spawn_worker(runtime_exe: &Path) -> bool {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        Command::new(runtime_exe)
            .arg(WORKER_ARG)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .is_ok()
    }

    fn hold_delete_share_handle(path: &Path) -> bool {
        use std::os::windows::ffi::OsStrExt;

        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);

        unsafe {
            let handle = CreateFileW(
                PCWSTR(wide.as_ptr()),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            );

            let Ok(handle) = handle else {
                return false;
            };

            if handle.is_invalid() {
                return false;
            }

            let _ = Box::leak(Box::new(DeleteShareHandle(handle)));
            true
        }
    }
}

#[cfg(windows)]
pub use imp::{enable_delete_while_running, relaunch_as_runtime_worker};

#[cfg(not(windows))]
pub fn relaunch_as_runtime_worker() -> bool {
    false
}

#[cfg(not(windows))]
pub fn enable_delete_while_running() -> bool {
    false
}
