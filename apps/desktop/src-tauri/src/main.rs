#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    #[cfg(windows)]
    anansi_desktop_lib::run();
    #[cfg(not(windows))]
    eprintln!("Desktop packaging is currently supported on Windows only; use the browser edition.");
}
