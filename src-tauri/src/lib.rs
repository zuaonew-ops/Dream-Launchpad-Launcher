use serde::Serialize;

#[derive(Serialize)]
struct PlatformInfo {
    os: String,
    arch: String,
    tauri: String,
    launcher: String,
    core: String,
    game_dir: String,
}

#[tauri::command]
fn platform_info() -> PlatformInfo {
    PlatformInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        tauri: tauri::VERSION.to_string(),
        launcher: env!("CARGO_PKG_VERSION").to_string(),
        core: dlp_core::VERSION.to_string(),
        game_dir: dlp_core::default_game_dir().display().to_string(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![platform_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
