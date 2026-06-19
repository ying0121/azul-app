//! One running worker. A second launch signals the running instance to show its window.

#[cfg(windows)]
mod imp {
    use std::thread;
    use std::time::Duration;

    use tauri::{AppHandle, Manager};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{
        CreateEventW, CreateMutexW, OpenEventW, OpenMutexW, SetEvent, WaitForSingleObject,
        INFINITE, SYNCHRONIZATION_ACCESS_RIGHTS,
    };

    const MUTEX_NAME: &str = "Local\\com.dailyhuddle.desktop.instance";
    const SHOW_EVENT_NAME: &str = "Local\\com.dailyhuddle.desktop.show";
    const EVENT_MODIFY_STATE: u32 = 0x0002;

    pub struct InstanceGuard(pub HANDLE);

    impl Drop for InstanceGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    struct LeakedEvent(HANDLE);

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

    pub fn notify_existing_instance_show() -> bool {
        for _ in 0..30 {
            if signal_show_event() {
                return true;
            }
            thread::sleep(Duration::from_millis(100));
        }
        false
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

    pub fn detach_working_directory(runtime_dir: &std::path::Path) {
        let _ = std::env::set_current_dir(runtime_dir);
    }

    pub fn start_show_listener(app: AppHandle) {
        let Some(event) = (unsafe { create_show_event() }) else {
            return;
        };

        let _ = Box::leak(Box::new(LeakedEvent(event)));
        let event_raw = event.0 as isize;

        thread::spawn(move || {
            let event = HANDLE(event_raw as *mut _);
            loop {
                unsafe {
                    let waited = WaitForSingleObject(event, INFINITE);
                    if waited != WAIT_OBJECT_0 {
                        continue;
                    }
                }

                let app = app.clone();
                let _ = app.clone().run_on_main_thread(move || {
                    show_main_window(&app);
                });
            }
        });
    }

    fn show_main_window(app: &AppHandle) {
        let Some(window) = app.get_webview_window("main") else {
            return;
        };

        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }

    fn signal_show_event() -> bool {
        unsafe {
            let name = wide(SHOW_EVENT_NAME);
            let Ok(event) = OpenEventW(
                SYNCHRONIZATION_ACCESS_RIGHTS(0x0010_0000 | EVENT_MODIFY_STATE),
                false,
                PCWSTR(name.as_ptr()),
            ) else {
                return false;
            };

            let signaled = SetEvent(event).is_ok();
            let _ = CloseHandle(event);
            signaled
        }
    }

    unsafe fn create_show_event() -> Option<HANDLE> {
        let name = wide(SHOW_EVENT_NAME);
        let handle = CreateEventW(None, false, false, PCWSTR(name.as_ptr())).ok()?;
        if handle.is_invalid() {
            None
        } else {
            Some(handle)
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(windows)]
pub use imp::{
    acquire_instance_guard, detach_working_directory, is_another_instance_running,
    notify_existing_instance_show, start_show_listener, InstanceGuard,
};

#[cfg(not(windows))]
pub fn is_another_instance_running() -> bool {
    false
}

#[cfg(not(windows))]
pub fn notify_existing_instance_show() -> bool {
    false
}

#[cfg(not(windows))]
pub fn acquire_instance_guard() -> Option<()> {
    Some(())
}

#[cfg(not(windows))]
pub fn detach_working_directory(_runtime_dir: &std::path::Path) {}

#[cfg(not(windows))]
pub fn start_show_listener(_app: tauri::AppHandle) {}
