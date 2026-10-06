#![cfg_attr(not(test), windows_subsystem = "windows")]
#[cfg(windows)]
mod desktop;
mod dos;
mod game;
fn main() {
    #[cfg(windows)]
    desktop::run();
    #[cfg(not(windows))]
    eprintln!("This edition of Vlak targets Windows. Build on Windows with cargo build --release.");
}
