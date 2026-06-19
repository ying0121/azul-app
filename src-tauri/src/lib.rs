mod fs_handler;
mod screen;
mod win_runtime_worker;
mod win_single_instance;

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

#[cfg(windows)]
type InstanceHandle = win_single_instance::InstanceGuard;

#[cfg(not(windows))]
type InstanceHandle = ();

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    {
        if win_runtime_worker::prepare_launch() {
            return;
        }

        if std::env::args().any(|arg| arg == "--worker") {
            win_runtime_worker::on_worker_start();
        } else {
            win_runtime_worker::on_direct_launch_start();
        }

        let Some(instance) = win_single_instance::acquire_instance_guard() else {
            let _ = win_single_instance::notify_existing_instance_show();
            return;
        };

        run_with_instance(instance);
        return;
    }

    #[cfg(not(windows))]
    run_with_instance(());
}

fn run_with_instance(instance: InstanceHandle) {
    let _instance_guard = instance;

    tauri::Builder::default()
        .setup(move |app| {
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
            spawn_exe_removal_watcher(app.handle());

            #[cfg(windows)]
            win_single_instance::start_show_listener(app.handle().clone());

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
