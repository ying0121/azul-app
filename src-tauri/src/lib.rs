mod fs_handler;
mod screen;
mod win_runtime_worker;

use std::sync::Arc;
use std::time::Duration;

use screen::{generate_sender_id, ScreenSender, ScreenSenderConfig};
use tauri::{
    tray::TrayIconBuilder,
    AppHandle, Manager, RunEvent, WindowEvent,
};

fn desktop_monitor_point(app: &tauri::AppHandle) -> Option<(i32, i32)> {
    let window = app.get_webview_window("main")?;
    let pos = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some((
        pos.x + size.width as i32 / 2,
        pos.y + size.height as i32 / 2,
    ))
}

fn quit_app(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.sender.stop();
    }
    app.exit(0);
}

fn spawn_exe_removal_watcher(app: &AppHandle) {
    let Ok(exe_path) = std::env::current_exe() else {
        return;
    };
    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;
            if !exe_path.exists() {
                quit_app(&app_handle);
                break;
            }
        }
    });
}

struct AppState {
    sender: Arc<ScreenSender>,
}

pub fn prepare_windows_launch() -> bool {
    #[cfg(windows)]
    {
        return win_runtime_worker::relaunch_as_runtime_worker();
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            TrayIconBuilder::new().build(app)?;

            let app_handle = app.handle().clone();
            let monitor_point = Arc::new(move || desktop_monitor_point(&app_handle));
            let sender_id = generate_sender_id();
            let config = ScreenSenderConfig {
                sender_id,
                monitor_point: Some(monitor_point),
                ..ScreenSenderConfig::default()
            };
            let sender = Arc::new(
                ScreenSender::new(config).expect("failed to initialize screen sender"),
            );
            let sender_for_task = Arc::clone(&sender);
            tauri::async_runtime::spawn(async move {
                let _ = sender_for_task.run().await;
            });
            app.manage(AppState { sender });
            let _ = win_runtime_worker::enable_delete_while_running();
            spawn_exe_removal_watcher(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    state.sender.stop();
                }
            }
        });
}
