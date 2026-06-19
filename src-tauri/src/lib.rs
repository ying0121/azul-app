mod screen;

use std::sync::Arc;

use screen::{load_or_create_sender_id, ScreenSender, ScreenSenderConfig};
use tauri::Manager;

fn desktop_monitor_point(app: &tauri::AppHandle) -> Option<(i32, i32)> {
    let window = app.get_webview_window("main")?;
    let pos = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some((
        pos.x + size.width as i32 / 2,
        pos.y + size.height as i32 / 2,
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            let monitor_point = Arc::new(move || desktop_monitor_point(&app_handle));
            let app_data_dir = app.path().app_data_dir().ok();
            let sender_id = load_or_create_sender_id(app_data_dir);
            let config = ScreenSenderConfig {
                sender_id,
                monitor_point: Some(monitor_point),
                ..ScreenSenderConfig::default()
            };
            match ScreenSender::new(config) {
                Ok(sender) => {
                    tauri::async_runtime::spawn(async move {
                        let _ = sender.run().await;
                    });
                }
                Err(_err) => {}
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
