pub(crate) mod commands;
pub(crate) mod launch_mode;
pub(crate) mod show_signal;
pub(crate) mod single_instance;

use tauri::{AppHandle, Manager, WebviewWindow};

pub fn get_main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "Main window is unavailable.".to_string())
}

pub fn show_main_window(app: &AppHandle) {
    if let Ok(window) = get_main_window(app) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn hide_main_window(app: &AppHandle) {
    if let Ok(window) = get_main_window(app) {
        let _ = window.hide();
    }
}
