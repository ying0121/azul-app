use tauri::AppHandle;

use crate::window_shell;

#[tauri::command]
pub fn window_minimize(app: AppHandle) -> Result<(), String> {
    window_shell::get_main_window(&app)?
        .minimize()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn window_toggle_maximize(app: AppHandle) -> Result<(), String> {
    let window = window_shell::get_main_window(&app)?;
    if window.is_maximized().map_err(|error| error.to_string())? {
        window.unmaximize().map_err(|error| error.to_string())
    } else {
        window.maximize().map_err(|error| error.to_string())
    }
}

#[tauri::command]
pub fn window_hide(app: AppHandle) -> Result<(), String> {
    window_shell::hide_main_window(&app);
    Ok(())
}

#[tauri::command]
pub fn window_is_maximized(app: AppHandle) -> Result<bool, String> {
    window_shell::get_main_window(&app)?
        .is_maximized()
        .map_err(|error| error.to_string())
}
