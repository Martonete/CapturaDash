// Sin consola en compilaciones de release para Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    canvasclip_lib::run()
}
