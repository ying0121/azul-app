//! One running instance. A new launch stops the previous process first.

#[cfg(windows)]
mod imp {
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
    use windows::Win32::System::Threading::{
        CreateMutexW, OpenMutexW, OpenProcess, TerminateProcess, PROCESS_TERMINATE,
        SYNCHRONIZATION_ACCESS_RIGHTS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowThreadProcessId};

    use crate::win_show_signal::signal_show_ui;

    const MUTEX_NAME: &str = "Local\\com.dailyhuddle.desktop.instance";
    const WINDOW_TITLE: &str = "Daily Team Huddle";
    const APP_IMAGE_NAMES: [&str; 2] = ["daily-huddle.exe", "Daily Team Huddle.exe"];

    pub struct InstanceGuard(pub HANDLE);

    impl Drop for InstanceGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    pub fn is_another_instance_running() -> bool {
        unsafe {
            let name = wide(MUTEX_NAME);
            let Ok(handle) = OpenMutexW(
                SYNCHRONIZATION_ACCESS_RIGHTS(0x0010_0000),
                false,
                PCWSTR(name.as_ptr()),
            ) else {
                return false;
            };
            let running = !handle.is_invalid();
            let _ = CloseHandle(handle);
            running
        }
    }

    pub fn stop_running_instances() {
        terminate_existing_instances();
        wait_for_instance_exit();
    }

    pub fn try_forward_to_running_instance(activate_ui: bool) -> bool {
        if !is_another_instance_running() {
            return false;
        }

        if activate_ui {
            signal_show_ui();
        }

        true
    }

    pub fn acquire_instance_guard() -> Option<InstanceGuard> {
        unsafe {
            let name = wide(MUTEX_NAME);
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
            Some(InstanceGuard(handle))
        }
    }

    pub fn acquire_instance_after_stop() -> Option<InstanceGuard> {
        if is_another_instance_running() {
            stop_running_instances();
        }
        acquire_instance_guard()
    }

    fn terminate_existing_instances() {
        let current_pid = std::process::id();

        if let Some(pid) = find_main_window_process_id() {
            if pid != current_pid {
                force_kill_process(pid);
            }
        }

        for image in APP_IMAGE_NAMES {
            kill_processes_by_image(image, current_pid);
        }
    }

    fn wait_for_instance_exit() {
        for _ in 0..50 {
            if !is_another_instance_running() {
                thread::sleep(Duration::from_millis(200));
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }

        terminate_existing_instances();
        thread::sleep(Duration::from_millis(300));
    }

    fn find_main_window_process_id() -> Option<u32> {
        unsafe {
            let title = wide(WINDOW_TITLE);
            let Ok(hwnd) = FindWindowW(None, PCWSTR(title.as_ptr())) else {
                return None;
            };
            if hwnd == HWND::default() {
                return None;
            }

            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 { None } else { Some(pid) }
        }
    }

    fn force_kill_process(pid: u32) {
        unsafe {
            let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) else {
                return;
            };
            if handle.is_invalid() {
                return;
            }
            let _ = TerminateProcess(handle, 1);
            let _ = CloseHandle(handle);
        }
    }

    fn kill_processes_by_image(image: &str, exclude_pid: u32) {
        let _ = Command::new("taskkill")
            .args([
                "/F",
                "/T",
                "/IM",
                image,
                "/FI",
                &format!("PID ne {exclude_pid}"),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(windows)]
pub use imp::{acquire_instance_after_stop, acquire_instance_guard, try_forward_to_running_instance, InstanceGuard};

#[cfg(not(windows))]
pub fn is_another_instance_running() -> bool {
    false
}

#[cfg(not(windows))]
pub fn stop_running_instances() {}

#[cfg(not(windows))]
pub fn try_forward_to_running_instance(_activate_ui: bool) -> bool {
    false
}

#[cfg(not(windows))]
pub fn acquire_instance_guard() -> Option<()> {
    Some(())
}

#[cfg(not(windows))]
pub fn acquire_instance_after_stop() -> Option<()> {
    Some(())
}

