// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if daily_huddle_lib::prepare_windows_launch() {
        return;
    }

    daily_huddle_lib::run();
}
