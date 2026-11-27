// pas de console sur Windows en version finale
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

fn main() {
    argos_tauri_lib::run()
}
