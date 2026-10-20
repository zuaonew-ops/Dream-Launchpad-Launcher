use dlp_core::{install, java, launch, minecraft, net::Progress, paths::Paths};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{Emitter, State};

struct AppState {
    paths: Paths,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct VersionSummary {
    id: String,
    kind: String,
    release_time: String,
    installed: bool,
}

#[derive(Serialize, Clone)]
struct ProgressPayload {
    stage: String,
    done: usize,
    total: usize,
}

#[derive(Serialize)]
struct Environment {
    os: String,
    arch: String,
    tauri: String,
    core: String,
    data_dir: String,
}

#[tauri::command]
async fn environment(state: State<'_, AppState>) -> Result<Environment, String> {
    Ok(Environment {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        tauri: tauri::VERSION.to_string(),
        core: dlp_core::VERSION.to_string(),
        data_dir: state.paths.root.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn list_versions(
    state: State<'_, AppState>,
    include_snapshots: bool,
) -> Result<Vec<VersionSummary>, String> {
    let manifest = minecraft::fetch(&state.client).await?;
    Ok(manifest
        .versions
        .into_iter()
        .filter(|v| include_snapshots || v.is_release())
        .map(|v| VersionSummary {
            installed: state.paths.is_installed(&v.id),
            id: v.id,
            kind: v.kind,
            release_time: v.release_time,
        })
        .collect())
}

#[tauri::command]
async fn detect_java() -> Result<Vec<java::JavaInfo>, String> {
    tauri::async_runtime::spawn_blocking(java::detect)
        .await
        .map_err(|e| format!("探测 Java 失败: {e}"))
}

#[tauri::command]
async fn install_version(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let paths = state.paths.clone();
    let client = state.client.clone();

    let manifest = minecraft::fetch(&client).await?;
    let entry = manifest
        .versions
        .into_iter()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("找不到版本 {id}"))?;
    install::ensure_version_json(&client, &paths, &entry).await?;

    let progress = Progress::new(move |stage, done, total| {
        let _ = app.emit(
            "install-progress",
            ProgressPayload {
                stage: stage.to_string(),
                done,
                total,
            },
        );
    });

    install::install(&client, &paths, &id, progress).await
}

#[tauri::command]
async fn launch_game(
    state: State<'_, AppState>,
    id: String,
    username: String,
    memory_mb: u32,
    java_path: Option<String>,
    isolate: bool,
) -> Result<u32, String> {
    let paths = state.paths.clone();
    let version = minecraft::load_version(&paths, &id)?;
    let required = version
        .java_version
        .as_ref()
        .map(|j| j.major_version)
        .unwrap_or(8);

    let java_bin = match java_path {
        Some(path) => PathBuf::from(path),
        None => {
            let found = tauri::async_runtime::spawn_blocking(java::detect)
                .await
                .map_err(|e| format!("探测 Java 失败: {e}"))?;
            java::pick(&found, required)
                .map(|j| PathBuf::from(&j.path))
                .ok_or_else(|| format!("没有找到 Java {required} 或更高版本，请先安装"))?
        }
    };

    let options = launch::LaunchOptions {
        id: &id,
        username: &username,
        java: &java_bin,
        memory_mb,
        isolate,
    };
    let plan = launch::build(&paths, &version, &options)?;
    launch::spawn(&plan, &paths)
}

#[tauri::command]
fn open_dir(state: State<'_, AppState>, kind: String) -> Result<(), String> {
    let base = state.paths.root.clone();
    let dir = match kind.as_str() {
        "logs" => base.join("logs"),
        "versions" => base.join("versions"),
        _ => base,
    };
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {e}"))?;
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = std::process::Command::new("explorer");
        c.arg(&dir);
        c
    };
    #[cfg(target_os = "linux")]
    let mut command = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(&dir);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = std::process::Command::new("open");
        c.arg(&dir);
        c
    };
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("打开目录失败: {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let paths = Paths::new(Paths::default_root());
    std::fs::create_dir_all(&paths.root).ok();

    let client = reqwest::Client::builder()
        .user_agent(concat!("DLP/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("构建 HTTP 客户端失败");

    tauri::Builder::default()
        .manage(AppState { paths, client })
        .invoke_handler(tauri::generate_handler![
            environment,
            list_versions,
            detect_java,
            install_version,
            launch_game,
            open_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
