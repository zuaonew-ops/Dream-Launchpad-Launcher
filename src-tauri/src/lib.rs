use dlp_core::{
    accounts::Accounts, content, install, java, launch, loader, market, minecraft,
    net::Progress,
    paths::Paths,
    runtime,
    settings::{AppSettings, Store},
};
use std::collections::HashSet;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};

struct AppState {
    paths: Paths,
    client: reqwest::Client,
    /// 正在运行的游戏进程（退出后清空）
    game: Mutex<Option<RunningGame>>,
}

#[derive(Serialize)]
struct VersionSummary {
    id: String,
    kind: String,
    release_time: String,
    installed: bool,
    /// true = 只存在于本地 versions/ 目录（别的启动器装的 / 手动放的）
    local: bool,
    /// 愚人节快照（清单里仍标 release/snapshot，只能认 id）
    april_fools: bool,
    /// 从版本 id 推断的加载器（fabric / quilt / forge / neoforge），原版为 None
    loader: Option<String>,
}

#[derive(Serialize, Clone)]
struct ProgressPayload {
    stage: String,
    done: usize,
    total: usize,
}

#[derive(Serialize, Clone)]
struct RunningGame {
    pid: u32,
    id: String,
}

#[derive(Serialize, Clone)]
struct GameExit {
    pid: u32,
    id: String,
    code: Option<i32>,
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
    let paths = state.paths.clone();
    let manifest = minecraft::fetch(&state.client).await?;
    let local = minecraft::scan::scan(&paths);
    let installable: HashSet<&str> = local
        .iter()
        .filter(|v| v.has_json)
        .map(|v| v.id.as_str())
        .collect();

    let mut out: Vec<VersionSummary> = manifest
        .versions
        .into_iter()
        .filter(|v| include_snapshots || v.is_release())
        .map(|v| VersionSummary {
            installed: installable.contains(v.id.as_str()) || paths.is_installed(&v.id),
            april_fools: minecraft::is_april_fools(&v.id),
            loader: loader_of(&v.id),
            id: v.id,
            kind: v.kind,
            release_time: v.release_time,
            local: false,
        })
        .collect();

    // 清单里没有的本地版本也要显示出来，否则别的启动器装的游戏看不见
    let listed: HashSet<String> = out.iter().map(|v| v.id.clone()).collect();
    for version in local {
        if listed.contains(&version.id) || !version.has_json {
            continue;
        }
        let april_fools = minecraft::is_april_fools(&version.id);
        let loader = loader_of(&version.id);
        out.push(VersionSummary {
            id: version.id,
            kind: version.kind,
            release_time: version.release_time,
            installed: true,
            local: true,
            april_fools,
            loader,
        });
    }

    Ok(out)
}

#[tauri::command]
fn running_game(state: State<'_, AppState>) -> Option<RunningGame> {
    state.game.lock().unwrap().clone()
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
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    username: String,
) -> Result<u32, String> {
    let paths = state.paths.clone();
    let version = minecraft::load_version(&paths, &id)?;
    // 每个版本有自己的设置，读不到就用全局默认
    let settings = Store::load(&paths).merged(&id);
    let required = version
        .java_version
        .as_ref()
        .map(|j| j.major_version)
        .unwrap_or(8);

    let java_bin = match settings.java_path.clone() {
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
        memory_mb: settings.memory_mb.unwrap_or(2048),
        isolate: settings.isolate.unwrap_or(true),
        extra_jvm_args: &settings.extra_jvm_args,
        game_args: &settings.game_args,
    };
    let plan = launch::build(&paths, &version, &options)?;

    if let Some(existing) = state.game.lock().unwrap().as_ref() {
        return Err(format!("游戏已在运行（pid {}）", existing.pid));
    }

    let mut child = launch::spawn(&plan, &paths)?;
    let pid = child.id();
    *state.game.lock().unwrap() = Some(RunningGame {
        pid,
        id: id.clone(),
    });

    // 等待进程退出：清掉状态并通知前端，否则界面会一直显示「运行中」
    let handle = app.clone();
    std::thread::spawn(move || {
        let status = child.wait();
        if let Some(state) = handle.try_state::<AppState>() {
            let mut guard = state.game.lock().unwrap();
            if guard.as_ref().map(|g| g.pid) == Some(pid) {
                *guard = None;
            }
        }
        let _ = handle.emit(
            "game-exit",
            GameExit {
                pid,
                id,
                code: status.ok().and_then(|s| s.code()),
            },
        );
    });

    Ok(pid)
}

#[tauri::command]
fn scan_versions(state: State<'_, AppState>) -> Vec<minecraft::LocalVersion> {
    minecraft::scan::scan(&state.paths)
}

#[tauri::command]
fn version_settings(state: State<'_, AppState>, id: String) -> dlp_core::settings::VersionSettings {
    Store::load(&state.paths).merged(&id)
}

#[tauri::command]
fn save_version_settings(
    state: State<'_, AppState>,
    id: String,
    settings: dlp_core::settings::VersionSettings,
) -> Result<(), String> {
    let mut store = Store::load(&state.paths);
    store.set(&id, settings);
    store.save(&state.paths)
}

#[tauri::command]
async fn loader_candidates(
    state: State<'_, AppState>,
    mc: String,
    kind: String,
) -> Result<Vec<loader::Candidate>, String> {
    let kind = loader::LoaderKind::parse(&kind).ok_or_else(|| format!("未知加载器: {kind}"))?;
    loader::candidates(&state.client, &mc, kind).await
}

#[tauri::command]
async fn optifine_candidates(
    state: State<'_, AppState>,
    mc: String,
) -> Result<Vec<loader::Candidate>, String> {
    loader::optifine_candidates(&state.client, &mc).await
}

#[tauri::command]
async fn install_loader(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    mc: String,
    kind: String,
    version: String,
) -> Result<String, String> {
    let kind = loader::LoaderKind::parse(&kind).ok_or_else(|| format!("未知加载器: {kind}"))?;
    let paths = state.paths.clone();
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

    match kind {
        loader::LoaderKind::Fabric | loader::LoaderKind::Quilt => {
            loader::install_meta(&state.client, &paths, &mc, kind, &version, Some(progress)).await
        }
        _ => {
            let java = pick_java(&paths, &mc)?;
            let installer =
                loader::resolve_installer_path(&state.client, &mc, kind, &version).await;
            loader::install_with_installer(
                &state.client,
                &paths,
                &mc,
                kind,
                &version,
                installer,
                &java,
                Some(progress),
            )
            .await
        }
    }
}

#[tauri::command]
async fn install_optifine(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    mc: String,
    edition: String,
) -> Result<String, String> {
    let paths = state.paths.clone();
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
    let java = pick_java(&paths, &mc)?;
    loader::install_optifine(&state.client, &paths, &mc, &edition, &java, Some(progress)).await
}

/// 挑一个能跑该版本的 Java（安装器 / OptiFine 用）
fn pick_java(paths: &Paths, mc: &str) -> Result<PathBuf, String> {
    let required = minecraft::load_version(paths, mc)
        .ok()
        .and_then(|v| v.java_version.as_ref().map(|j| j.major_version))
        .unwrap_or(8);
    let found = java::detect();
    java::pick(&found, required)
        .map(|j| PathBuf::from(&j.path))
        .ok_or_else(|| format!("没有找到 Java {required} 或更高版本"))
}

#[tauri::command]
fn list_mods(state: State<'_, AppState>, id: String) -> Vec<content::ModEntry> {
    content::list_mods(&game_dir(&state.paths, &id))
}

#[tauri::command]
fn set_mod_enabled(
    state: State<'_, AppState>,
    id: String,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    content::set_mod_enabled(&game_dir(&state.paths, &id), &name, enabled)
}

#[tauri::command]
fn delete_mod(state: State<'_, AppState>, id: String, name: String) -> Result<(), String> {
    content::delete_mod(&game_dir(&state.paths, &id), &name)
}

#[tauri::command]
fn list_saves(state: State<'_, AppState>, id: String) -> Vec<content::SaveEntry> {
    content::list_saves(&game_dir(&state.paths, &id))
}

#[tauri::command]
fn list_logs(state: State<'_, AppState>) -> Vec<String> {
    content::list_logs(&state.paths)
}

#[tauri::command]
fn read_log(state: State<'_, AppState>, name: String, lines: usize) -> Result<String, String> {
    content::read_log(&state.paths, &name, lines.max(20))
}

/// 打开该版本的游戏目录 / mods / saves
#[tauri::command]
async fn repair_version(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let paths = state.paths.clone();
    let client = state.client.clone();
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
fn delete_version(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let paths = state.paths.clone();
    if state.game.lock().unwrap().as_ref().map(|g| g.id.as_str()) == Some(id.as_str()) {
        return Err("这个版本正在运行，先关掉游戏".into());
    }
    let dir = paths.version_dir(&id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("删除 {} 失败: {e}", dir.display()))?;
    }
    // 隔离模式下还有一份实例目录
    let instance = game_dir(&paths, &id);
    if instance.exists() && instance != dir {
        let _ = std::fs::remove_dir_all(&instance);
    }
    Ok(())
}

#[tauri::command]
fn version_stats(state: State<'_, AppState>, id: String) -> content::VersionStats {
    let isolate = Store::load(&state.paths).merged(&id).isolate.unwrap_or(false);
    content::version_stats(&state.paths, &id, isolate)
}

#[tauri::command]
fn all_mods(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<usize, String> {
    let dir = game_dir(&state.paths, &id).join("mods");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建 mods 目录失败: {e}"))?;
    content::set_all_mods(&dir, enabled)
}

#[tauri::command]
fn crash_reports(state: State<'_, AppState>) -> Vec<content::CrashEntry> {
    content::list_crashes(&state.paths)
}

#[tauri::command]
fn read_crash(state: State<'_, AppState>, name: String) -> Result<String, String> {
    content::read_crash(&state.paths, &name)
}

#[tauri::command]
fn accounts(state: State<'_, AppState>) -> Accounts {
    Accounts::load(&state.paths)
}

#[tauri::command]
fn add_account(state: State<'_, AppState>, name: String) -> Result<Accounts, String> {
    let mut acc = Accounts::load(&state.paths);
    acc.add(&name)?;
    acc.save(&state.paths)?;
    Ok(acc)
}

#[tauri::command]
fn remove_account(state: State<'_, AppState>, id: String) -> Result<Accounts, String> {
    let mut acc = Accounts::load(&state.paths);
    acc.remove(&id);
    acc.save(&state.paths)?;
    Ok(acc)
}

#[tauri::command]
fn use_account(state: State<'_, AppState>, id: String) -> Result<Accounts, String> {
    let mut acc = Accounts::load(&state.paths);
    acc.set_current(&id)?;
    acc.save(&state.paths)?;
    Ok(acc)
}

#[tauri::command]
fn app_settings(state: State<'_, AppState>) -> AppSettings {
    AppSettings::load(&state.paths)
}

#[tauri::command]
fn set_default_version(
    state: State<'_, AppState>,
    id: Option<String>,
) -> Result<AppSettings, String> {
    let mut cfg = AppSettings::load(&state.paths);
    cfg.default_version = id;
    cfg.save(&state.paths)?;
    Ok(cfg)
}

#[tauri::command]
fn open_version_dir(state: State<'_, AppState>, id: String, kind: String) -> Result<(), String> {
    // "version"（默认）= versions/<id> 本体；其余指向实例目录里的子目录
    let instance = game_dir(&state.paths, &id);
    let target = match kind.as_str() {
        "mods" => instance.join("mods"),
        "saves" => instance.join("saves"),
        "instance" => instance,
        _ => state.paths.version_dir(&id),
    };
    std::fs::create_dir_all(&target).map_err(|e| format!("创建目录失败: {e}"))?;
    open_path(&target)
}

fn game_dir(paths: &Paths, id: &str) -> PathBuf {
    let settings = Store::load(paths).merged(id);
    content::game_dir(paths, id, settings.isolate.unwrap_or(true))
}

fn open_path(dir: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = std::process::Command::new("explorer");
        c.arg(dir);
        c
    };
    #[cfg(target_os = "linux")]
    let mut command = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(dir);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = std::process::Command::new("open");
        c.arg(dir);
        c
    };
    command.spawn().map(|_| ()).map_err(|e| format!("打开目录失败: {e}"))
}

/// 从版本 id 猜加载器（Modrinth 搜索要用）
fn loader_of(id: &str) -> Option<String> {
    let lower = id.to_ascii_lowercase();
    if lower.contains("neoforge") {
        Some("neoforge".into())
    } else if lower.contains("fabric") {
        Some("fabric".into())
    } else if lower.contains("quilt") {
        Some("quilt".into())
    } else if lower.contains("forge") {
        Some("forge".into())
    } else {
        None
    }
}

/// 版本 id → 对应的原版版本号（fabric-loader-0.19.5-26.3 → 26.3）
fn base_mc_version(paths: &Paths, id: &str) -> String {
    minecraft::load_version(paths, id)
        .ok()
        .and_then(|v| v.inherits_from.clone())
        .unwrap_or_else(|| id.to_string())
}

#[tauri::command]
fn base_version(state: State<'_, AppState>, id: String) -> String {
    base_mc_version(&state.paths, &id)
}

#[tauri::command]
async fn search_market(
    state: State<'_, AppState>,
    query: String,
    kind: String,
    mc: Option<String>,
    loader: Option<String>,
    category: Option<String>,
    offset: u64,
    limit: u64,
) -> Result<market::SearchPage, String> {
    market::search(
        &state.client,
        &query,
        &kind,
        mc.as_deref(),
        loader.as_deref(),
        category.as_deref(),
        offset,
        limit,
    )
    .await
}

#[tauri::command]
async fn market_files(
    state: State<'_, AppState>,
    project: String,
    mc: Option<String>,
    loader: Option<String>,
) -> Result<Vec<market::FileChoice>, String> {
    market::files(&state.client, &project, mc.as_deref(), loader.as_deref()).await
}

/// 把资源装进该版本的游戏目录，返回文件名
#[tauri::command]
async fn install_market(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    project: String,
    kind: String,
) -> Result<String, String> {
    let paths = state.paths.clone();
    let dir = game_dir(&paths, &id);
    let mc = base_mc_version(&paths, &id);
    let loader = loader_of(&id);
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

    market::install(
        &state.client,
        &dir,
        &project,
        &kind,
        Some(&mc),
        loader.as_deref(),
        Some(progress),
    )
    .await
}

#[tauri::command]
async fn install_java(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    major: u32,
) -> Result<String, String> {
    let paths = state.paths.clone();
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
    let path = runtime::install(&state.client, &paths, major, Some(progress)).await?;
    Ok(path.to_string_lossy().to_string())
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
        .manage(AppState {
            paths,
            client,
            game: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            environment,
            list_versions,
            detect_java,
            install_version,
            launch_game,
            running_game,
            scan_versions,
            version_settings,
            save_version_settings,
            loader_candidates,
            optifine_candidates,
            install_loader,
            install_optifine,
            list_mods,
            set_mod_enabled,
            delete_mod,
            list_saves,
            list_logs,
            read_log,
            accounts,
            add_account,
            remove_account,
            use_account,
            app_settings,
            set_default_version,
            repair_version,
            delete_version,
            all_mods,
            version_stats,
            crash_reports,
            read_crash,
            open_version_dir,
            base_version,
            search_market,
            market_files,
            install_market,
            install_java,
            open_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
