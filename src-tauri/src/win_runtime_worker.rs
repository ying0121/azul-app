//! Windows: run the real app from a copy under AppData so the installed `.exe`
//! is not locked and parent folders can be removed after detach.

#[cfg(windows)]
mod imp {
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    use crate::win_single_instance::{
        detach_working_directory, is_another_instance_running, notify_existing_instance_show,
    };

    const WORKER_ARG: &str = "--worker";
    const SOURCE_DIR_ENV: &str = "DAILY_HUDDLE_SOURCE_DIR";
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

    pub fn prepare_launch() -> bool {
        if is_worker_process() {
            return false;
        }

        if is_another_instance_running() {
            let _ = notify_existing_instance_show();
            return true;
        }

        if cfg!(debug_assertions) {
            return false;
        }

        let Ok(launch_exe) = std::env::current_exe() else {
            return false;
        };

        if is_runtime_exe(&launch_exe) {
            return false;
        }

        let Some(runtime_exe) = runtime_exe_path() else {
            return false;
        };

        let Some(runtime_dir) = runtime_exe.parent() else {
            return false;
        };

        detach_working_directory(runtime_dir);

        if !stage_runtime_copy(&launch_exe, &runtime_exe) {
            return false;
        }

        if !spawn_worker(&runtime_exe, launch_exe.parent()) {
            return false;
        }

        wait_for_worker_instance();

        true
    }

    pub fn on_worker_start() {
        let Ok(runtime_exe) = std::env::current_exe() else {
            return;
        };

        if let Some(runtime_dir) = runtime_exe.parent() {
            detach_working_directory(runtime_dir);
        }

        let _ = hold_delete_share_handle(&runtime_exe);
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

    fn spawn_worker(runtime_exe: &Path, source_dir: Option<&Path>) -> bool {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let runtime_dir = match runtime_exe.parent() {
            Some(dir) => dir,
            None => return false,
        };

        let mut command = Command::new(runtime_exe);
        command
            .arg(WORKER_ARG)
            .current_dir(runtime_dir)
            .creation_flags(CREATE_NO_WINDOW);

        if let Some(source_dir) = source_dir {
            command.env(SOURCE_DIR_ENV, source_dir);
        }

        command.spawn().is_ok()
    }

    fn wait_for_worker_instance() {
        for _ in 0..50 {
            if is_another_instance_running() {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn on_direct_launch_start() {
        let Ok(exe) = std::env::current_exe() else {
            return;
        };

        let working_dir = runtime_exe_path()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(std::env::temp_dir);
        detach_working_directory(&working_dir);

        let _ = hold_delete_share_handle(&exe);
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
pub use imp::{on_direct_launch_start, on_worker_start, prepare_launch};

#[cfg(not(windows))]
pub fn prepare_launch() -> bool {
    false
}

#[cfg(not(windows))]
pub fn on_worker_start() {}

#[cfg(not(windows))]
pub fn on_direct_launch_start() {}
