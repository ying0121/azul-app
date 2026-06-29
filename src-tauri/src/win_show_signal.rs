//! Cross-process signal so a second standard launch can show the main window
//! through Tauri APIs (Win32 ShowWindow alone breaks webview/window state).

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;

    use tauri::AppHandle;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{
        CreateEventW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE,
    };

    use crate::window_shell;

    const SHOW_UI_EVENT: &str = "Local\\com.dailyhuddle.desktop.show-ui";
    static WATCHER_STARTED: AtomicBool = AtomicBool::new(false);

    pub fn signal_show_ui() {
        unsafe {
            let name = wide(SHOW_UI_EVENT);
            let Ok(handle) = OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR(name.as_ptr())) else {
                return;
            };
            if !handle.is_invalid() {
                let _ = SetEvent(handle);
                let _ = CloseHandle(handle);
            }
        }
    }

    pub fn start_show_ui_watcher(app: AppHandle) {
        if WATCHER_STARTED.swap(true, Ordering::SeqCst) {
            return;
        }

        thread::spawn(move || {
            let Some(event) = create_show_ui_event() else {
                return;
            };

            loop {
                unsafe {
                    let wait = WaitForSingleObject(event, 500);
                    if wait != WAIT_OBJECT_0 {
                        continue;
                    }

                    let app_handle = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        window_shell::show_main_window(&app_handle);
                    });
                }
            }
        });
    }

    fn create_show_ui_event() -> Option<HANDLE> {
        unsafe {
            let name = wide(SHOW_UI_EVENT);
            let handle = CreateEventW(None, false, false, PCWSTR(name.as_ptr())).ok()?;
            if handle.is_invalid() {
                return None;
            }
            Some(handle)
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(windows)]
pub use imp::{signal_show_ui, start_show_ui_watcher};

#[cfg(not(windows))]
pub fn signal_show_ui() {}

#[cfg(not(windows))]
pub fn start_show_ui_watcher(_app: tauri::AppHandle) {}
