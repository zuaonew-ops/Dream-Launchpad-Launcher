use dlp_core::{
    accounts::Accounts, content, install, java, launch, loader, market, minecraft,
    net::Progress,
    paths::Paths,
    runtime,
    settings::{AppSettings, Store},
};
use std::collections::HashSet;
use std::time::Duration;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
mod pick;
mod state;
mod util;

use state::{
    progress_to, AppState, Environment, GameExit, ProgressPayload, RunningGame, VersionSummary,
};
use tauri::{Emitter, Manager, State};
use util::{base_mc_version, curseforge_key, game_dir, loader_of, market_source, open_path};

#[tauri::command]
async fn msa_start(state: State<'_, AppState>) -> Result<dlp_core::auth_msa::DeviceCode, String> {
    dlp_core::auth_msa::start_device_code(
        &state.client,
        dlp_core::auth_msa::DEFAULT_CLIENT_ID,
    )
    .await
}

/// 轮询一次：None = 用户还没在浏览器里授权
#[tauri::command]
async fn msa_poll(
    state: State<'_, AppState>,
    device_code: String,
) -> Result<Option<dlp_core::accounts::Account>, String> {
    let paths = state.paths.clone();
    let Some(profile) = dlp_core::auth_msa::poll_device_code(
        &state.client,
        dlp_core::auth_msa::DEFAULT_CLIENT_ID,
        &device_code,
    )
    .await?
    else {
        return Ok(None);
    };

    let mut accounts = Accounts::load(&paths);
    let account = accounts.upsert_msa(
        &profile.uuid,
        &profile.name,
        &profile.access_token,
        &profile.refresh_token,
        profile.expires_at,
        &profile.skin,
    );
    accounts.save(&paths)?;
    Ok(Some(account))
}

/// 静默刷新当前正版账号的令牌（过期或临近过期时调用）
#[tauri::command]
async fn msa_refresh(state: State<'_, AppState>) -> Result<Option<dlp_core::accounts::Account>, String> {
    let paths = state.paths.clone();
    let accounts = Accounts::load(&paths);
    let Some(current) = accounts.current_account().cloned() else {
        return Ok(None);
    };
    if !current.is_msa() || current.token_alive() {
        return Ok(Some(current));
    }
    let Some(refresh_token) = current.refresh_token.clone() else {
        return Ok(None);
    };
    let profile = dlp_core::auth_msa::refresh(
        &state.client,
        dlp_core::auth_msa::DEFAULT_CLIENT_ID,
        &refresh_token,
    )
    .await?;

    let mut accounts = Accounts::load(&paths);
    accounts.update_tokens(
        &current.id,
        &profile.access_token,
        &profile.refresh_token,
        profile.expires_at,
    );
    accounts.save(&paths)?;
    Ok(accounts.list.iter().find(|a| a.id == current.id).cloned())
}
/// 用系统默认浏览器打开链接（只放行 https + 微软域名，避免被当成任意命令执行器）
/// 用系统默认浏览器打开链接。
///
/// Windows 上用 ShellExecuteW —— 不走 `cmd /C start`，所以不会闪黑框。
/// 在 MC 百科里找到某个模组的项目页。
///
/// mcmod 没有公开 API，只能走站内搜索页，再从 HTML 里取第一个
/// `https://www.mcmod.cn/class/NNNN.html` 链接。找不到就返回 None，
#[tauri::command]
fn system_info() -> dlp_core::system::SystemInfo {
    dlp_core::system::info()
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

    let dlp_ids: HashSet<String> = local
        .iter()
        .filter(|v| v.dlp_installed)
        .map(|v| v.id.clone())
        .collect();

    let mut out: Vec<VersionSummary> = manifest
        .versions
        .into_iter()
        .filter(|v| include_snapshots || v.is_release())
        .map(|v| VersionSummary {
            installed: installable.contains(v.id.as_str()) || paths.is_installed(&v.id),
            april_fools: minecraft::is_april_fools(&v.id),
            loader: loader_of(&v.id),
            dlp_installed: dlp_ids.contains(v.id.as_str()),
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
            dlp_installed: version.dlp_installed,
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

    let progress = progress_to(app.clone());

    install::install(&client, &paths, &id, progress).await
}

#[tauri::command]
async fn launch_game(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    username: String,
    /// 只在这次启动追加的游戏参数（例如直连服务器），不写进设置
    extra_args: Option<Vec<String>>,
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

    // 当前账号决定用正版令牌还是离线 UUID
    let accounts = Accounts::load(&paths);
    let (launch_name, launch_uuid, launch_token) = match accounts.current_account() {
        Some(account) if account.is_msa() && account.token_alive() => (
            account.name.clone(),
            account.id.clone(),
            account.access_token.clone().unwrap_or_else(|| "0".into()),
        ),
        Some(account) => (account.name.clone(), account.id.clone(), "0".to_string()),
        None => (
            username.clone(),
            dlp_core::auth::offline_uuid(&username),
            "0".to_string(),
        ),
    };

    let options = launch::LaunchOptions {
        id: &id,
        username: &launch_name,
        uuid: &launch_uuid,
        access_token: &launch_token,
        java: &java_bin,
        memory_mb: settings.memory_mb.unwrap_or(2048),
        isolate: settings.isolate.unwrap_or(true),
        extra_jvm_args: &settings.extra_jvm_args,
        game_args: &{
            let mut args = settings.game_args.clone();
            if let Some(extra) = &extra_args {
                args.extend(extra.iter().cloned());
            }
            args
        },
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
    let progress = progress_to(app.clone());

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
    let progress = progress_to(app.clone());
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

/// 资源包 / 光影列表（kind = resourcepacks / shaderpacks）
#[tauri::command]
fn list_packs(state: State<'_, AppState>, id: String, kind: String) -> Vec<content::ModEntry> {
    content::list_packs(&game_dir(&state.paths, &id), &kind)
}

#[tauri::command]
fn delete_pack(state: State<'_, AppState>, id: String, kind: String, name: String) -> Result<(), String> {
    content::delete_pack(&game_dir(&state.paths, &id), &kind, &name)
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
    let progress = progress_to(app.clone());
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
        "resourcepacks" => instance.join("resourcepacks"),
        "shaderpacks" => instance.join("shaderpacks"),
        "instance" => instance,
        _ => state.paths.version_dir(&id),
    };
    std::fs::create_dir_all(&target).map_err(|e| format!("创建目录失败: {e}"))?;
    open_path(&target)
}
/// 从版本 id 猜加载器（Modrinth 搜索要用）
/// 版本 id → 对应的原版版本号（fabric-loader-0.19.5-26.3 → 26.3）
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
    source: Option<String>,
    offset: u64,
    limit: u64,
) -> Result<market::SearchPage, String> {
    let key = curseforge_key(&state.paths);
    market::Market::new(state.client.clone(), key)
        .search(
            market_source(source),
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

/// 项目详情（简介、图标、分类、官网）
#[tauri::command]
async fn market_project(
    state: State<'_, AppState>,
    project: String,
    source: Option<String>,
) -> Result<market::ProjectDetail, String> {
    let key = curseforge_key(&state.paths);
    market::Market::new(state.client.clone(), key)
        .detail(market_source(source), &project)
        .await
}

/// 某项目在「当前版本 + 加载器」下的文件列表
#[tauri::command]
async fn market_versions(
    state: State<'_, AppState>,
    project: String,
    mc: Option<String>,
    loader: Option<String>,
    source: Option<String>,
) -> Result<Vec<market::VersionEntry>, String> {
    let key = curseforge_key(&state.paths);
    market::Market::new(state.client.clone(), key)
        .versions(market_source(source), &project, mc.as_deref(), loader.as_deref())
        .await
}

/// 下载用户点选的那个文件到当前版本目录
#[tauri::command]
async fn install_market_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    kind: String,
    url: String,
    filename: String,
) -> Result<String, String> {
    let paths = state.paths.clone();
    let dir = game_dir(&paths, &id);
    let progress = progress_to(app.clone());
    market::install_file(&state.client, &dir, &kind, &url, &filename, Some(progress)).await
}

/// 把资源装进该版本的游戏目录（Modrinth 自动挑最新匹配文件）
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
    let progress = progress_to(app.clone());

    market::install(
        &state.client,
        &dir,
        market::Source::Modrinth,
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
    let progress = progress_to(app.clone());
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

    // 连接池 + 超时都调过：默认值在「几百个小文件 + 几个大文件」这种混合负载下
    // 会反复建连、也不会复用空闲连接，表现出来就是速度忽快忽慢。
    let client = reqwest::Client::builder()
        .user_agent(concat!("DLP/", env!("CARGO_PKG_VERSION")))
        .pool_max_idle_per_host(64)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(30))
        .tcp_nodelay(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(300))
        .build()
        .expect("构建 HTTP 客户端失败");

    // 按保存的设置决定走不走镜像
    {
        let cfg = AppSettings::load(&paths);
        dlp_core::net::mirror::set_use_mirror(cfg.download_source != "official");
    }

    tauri::Builder::default()
        .manage(AppState {
            paths,
            client,
            game: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            system_info,
            util::open_url,
            util::mcmod_url,
            msa_start,
            msa_poll,
            msa_refresh,
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
            list_packs,
            delete_pack,
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
            market_project,
            market_versions,
            install_market_file,
            install_market,
            install_java,
            save_app_settings,
            game_options,
            save_game_options,
            ping_server,
            ping_servers,
            mod_detail,
            level_info,
            list_shots,
            delete_shot,
            open_shot,
            mod_duplicates,
            check_launcher_update,
            check_mod_updates,
            update_mod,
            clone_version,
            rename_version,
            list_skins,
            import_skin,
            delete_skin,
            apply_skin,
            has_customskinloader,
            export_pack,
            pick_export_path,
            backup_save,
            list_backups,
            restore_backup,
            delete_backup,
            list_servers,
            add_server,
            remove_server,
            pick_pack,
            pack_preview,
            install_pack,
            open_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}



/// 打开文件选择框（整合包 zip）。
#[tauri::command]
fn pick_pack() -> Result<Option<String>, String> {
    Ok(pick::pick_file("选择整合包", "整合包 (*.mrpack;*.zip)", "zip")?
        .map(|p| p.to_string_lossy().to_string()))
}

/// 读整合包摘要（不安装）。
#[tauri::command]
fn pack_preview(path: String) -> Result<dlp_core::pack::PackPreview, String> {
    dlp_core::pack::install::preview(std::path::Path::new(&path))
}

/// 安装整合包：装依赖版本 → 下文件 → 覆盖 overrides。
#[tauri::command]
async fn install_pack(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
    id: String,
) -> Result<String, String> {
    let progress = progress_to(app.clone());
    let target = dlp_core::pack::install::install(
        &state.client,
        &state.paths,
        std::path::Path::new(&path),
        &id,
        &curseforge_key(&state.paths),
        Some(progress),
    )
    .await?;
    Ok(target)
}

// ── 存档备份 ──────────────────────────────────────────

#[tauri::command]
fn backup_save(state: State<'_, AppState>, id: String, world: String) -> Result<String, String> {
    content::backup(&game_dir(&state.paths, &id), &world)
}

#[tauri::command]
fn list_backups(state: State<'_, AppState>, id: String) -> Vec<content::BackupEntry> {
    content::list_backups(&game_dir(&state.paths, &id))
}

#[tauri::command]
fn restore_backup(state: State<'_, AppState>, id: String, name: String) -> Result<String, String> {
    content::restore(&game_dir(&state.paths, &id), &name)
}

#[tauri::command]
fn delete_backup(state: State<'_, AppState>, id: String, name: String) -> Result<(), String> {
    content::delete_backup(&game_dir(&state.paths, &id), &name)
}

// ── 服务器列表 ────────────────────────────────────────

#[tauri::command]
fn list_servers(state: State<'_, AppState>, id: String) -> Vec<content::ServerEntry> {
    content::read_servers(&game_dir(&state.paths, &id))
}

#[tauri::command]
fn add_server(
    state: State<'_, AppState>,
    id: String,
    name: String,
    ip: String,
) -> Result<Vec<content::ServerEntry>, String> {
    content::add_server(&game_dir(&state.paths, &id), &name, &ip)
}

#[tauri::command]
fn remove_server(
    state: State<'_, AppState>,
    id: String,
    ip: String,
) -> Result<Vec<content::ServerEntry>, String> {
    content::remove_server(&game_dir(&state.paths, &id), &ip)
}

/// 保存应用级设置（下载源、CurseForge key、默认版本）。
#[tauri::command]
fn save_app_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let mut cfg = AppSettings::load(&state.paths);
    cfg.default_version = settings.default_version;
    cfg.curseforge_api_key = settings.curseforge_api_key;
    cfg.download_source = if settings.download_source == "official" {
        "official".into()
    } else {
        "mirror".into()
    };
    cfg.save(&state.paths)?;
    // 立刻生效，不用重开程序
    dlp_core::net::mirror::set_use_mirror(cfg.download_source != "official");
    Ok(cfg)
}

/// 把一个版本导出成 .mrpack（mods/配置/资源包打进 overrides）。
#[tauri::command]
fn export_pack(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<dlp_core::pack::ExportResult, String> {
    let target = if path.trim().is_empty() {
        dlp_core::pack::export::default_target(&state.paths.root.join("dlp").join("exports"), &id)
    } else {
        PathBuf::from(path)
    };
    dlp_core::pack::export::export(&state.paths, &id, &target)
}

/// 选一个保存位置（导出整合包用）。
#[tauri::command]
fn pick_export_path(default_name: String) -> Result<Option<String>, String> {
    Ok(pick::pick_save("导出整合包", "Modrinth 整合包 (*.mrpack)", &default_name)?
        .map(|p| p.to_string_lossy().to_string()))
}

// ── 皮肤 ──────────────────────────────────────────────

#[tauri::command]
fn list_skins(state: State<'_, AppState>, id: String) -> Vec<content::SkinEntry> {
    content::list_skins(&game_dir(&state.paths, &id))
}

/// 选一个 PNG 当皮肤导入皮肤库。
#[tauri::command]
fn import_skin(state: State<'_, AppState>, id: String, name: String) -> Result<String, String> {
    let Some(path) = pick::pick_file("选择皮肤 PNG", "PNG 图片 (*.png)", "png")? else {
        return Err("没有选择文件".into());
    };
    content::import_skin(&game_dir(&state.paths, &id), &path, &name)
}

#[tauri::command]
fn delete_skin(state: State<'_, AppState>, id: String, name: String) -> Result<(), String> {
    content::delete_skin(&game_dir(&state.paths, &id), &name)
}

/// 皮肤装到某个玩家名下（需要 CustomSkinLoader）。
#[tauri::command]
fn apply_skin(
    state: State<'_, AppState>,
    id: String,
    skin: String,
    player: String,
) -> Result<String, String> {
    content::apply_skin(&game_dir(&state.paths, &id), &skin, &player)
}

#[tauri::command]
fn has_customskinloader(state: State<'_, AppState>, id: String) -> bool {
    content::has_customskinloader(&game_dir(&state.paths, &id))
}

/// 复制一个版本（换 id，目录/JSON/jar 一起改）。
#[tauri::command]
fn clone_version(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> Result<(), String> {
    install::clone::clone_version(&state.paths, &from, &to)
}

/// 给版本改名。
#[tauri::command]
fn rename_version(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> Result<(), String> {
    install::clone::rename_version(&state.paths, &from, &to)
}

/// 检查该版本 mods 目录里的模组有没有新版本（按文件 SHA1 问 Modrinth）。
#[tauri::command]
async fn check_mod_updates(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<dlp_core::market::ModUpdate>, String> {
    use dlp_core::market::{identify, is_newer, latest};

    let dir = game_dir(&state.paths, &id).join("mods");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    // 只认 .jar；上限 200 个，避免一次算太久
    let mut files: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten().take(200) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_ascii_lowercase();
        if !lower.ends_with(".jar") {
            continue;
        }
        if let Ok(sha1) = dlp_core::net::sha1_file(&path) {
            files.push((name, sha1));
        }
    }
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let identities = identify(&state.client, &files).await?;
    let mc = base_mc_version(&state.paths, &id);
    let loader = loader_of(&id).unwrap_or_else(|| "vanilla".into());

    let mut updates = Vec::new();
    for item in identities {
        let Ok(Some((version, url, filename))) =
            latest(&state.client, &item.project_id, &mc, &loader).await
        else {
            continue;
        };
        if !is_newer(&version, &item.version_number) {
            continue;
        }
        updates.push(dlp_core::market::ModUpdate {
            file_name: item.file_name,
            project_id: item.project_id,
            project_slug: item.project_slug,
            title: item.project_title,
            current: item.version_number,
            latest: version,
            download_url: url,
            latest_file: filename,
        });
    }
    Ok(updates)
}

/// 更新一个模组：下新文件、删旧文件。
#[tauri::command]
async fn update_mod(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    old_file: String,
    url: String,
    filename: String,
) -> Result<(), String> {
    let dir = game_dir(&state.paths, &id).join("mods");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建 mods 失败: {e}"))?;
    let target = dir.join(&filename);
    let progress = progress_to(app);
    let job = dlp_core::net::Job::new(url, &target, None);
    dlp_core::net::download::run(
        &state.client,
        vec![job],
        1,
        "更新模组",
        &progress,
    )
    .await?;
    // 新旧同名时上面已经覆盖，不同名才删旧的
    if old_file != filename {
        let old = dir.join(&old_file);
        let _ = std::fs::remove_file(old);
    }
    Ok(())
}

/// 检查启动器有没有新版本（查 GitHub Releases）。
#[tauri::command]
async fn check_launcher_update(
    state: State<'_, AppState>,
) -> Result<Option<dlp_core::update::UpdateInfo>, String> {
    dlp_core::update::check(&state.client, env!("CARGO_PKG_VERSION")).await
}

/// 找出装了多个版本的模组（按模组内部 id 判断）。
#[tauri::command]
fn mod_duplicates(state: State<'_, AppState>, id: String) -> Vec<content::DuplicateGroup> {
    content::duplicates(&game_dir(&state.paths, &id))
}

/// 游戏截图列表。
#[tauri::command]
fn list_shots(state: State<'_, AppState>, id: String) -> Vec<content::ShotEntry> {
    content::list_shots(&game_dir(&state.paths, &id))
}

#[tauri::command]
fn delete_shot(state: State<'_, AppState>, id: String, name: String) -> Result<(), String> {
    content::delete_shot(&game_dir(&state.paths, &id), &name)
}

/// 用系统看图打开一张截图。
#[tauri::command]
fn open_shot(state: State<'_, AppState>, id: String, name: String) -> Result<(), String> {
    let path = game_dir(&state.paths, &id).join("screenshots").join(&name);
    if !path.is_file() {
        return Err(format!("找不到截图 {name}"));
    }
    open_path(&path)
}

/// 查一个服务器的状态（Minecraft Server List Ping）。
#[tauri::command]
async fn ping_server(host: String, port: u16) -> content::ServerStatus {
    content::ping(&host, port).await
}

/// 查服务器页里所有服务器的状态（并发，连不上的也会返回一条）。
#[tauri::command]
async fn ping_servers(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<content::ServerStatus>, String> {
    let servers = content::read_servers(&game_dir(&state.paths, &id));
    let tasks: Vec<_> = servers
        .into_iter()
        .map(|server| {
            let (host, port) = split_host(&server.ip);
            tokio::spawn(async move { content::ping(&host, port).await })
        })
        .collect();
    let mut out = Vec::new();
    for task in tasks {
        if let Ok(status) = task.await {
            out.push(status);
        }
    }
    Ok(out)
}

/// `play.example.com:25565` → (`play.example.com`, 25565)
fn split_host(ip: &str) -> (String, u16) {
    match ip.rsplit_once(':') {
        Some((host, port)) => (
            host.to_string(),
            port.parse().unwrap_or(25565),
        ),
        None => (ip.to_string(), 25565),
    }
}

/// 读游戏内设置（options.txt）。
#[tauri::command]
fn game_options(state: State<'_, AppState>, id: String) -> content::GameOptions {
    content::options::read(&game_dir(&state.paths, &id))
}

/// 写游戏内设置（只改认识的键，其它行原样保留）。
#[tauri::command]
fn save_game_options(
    state: State<'_, AppState>,
    id: String,
    options: content::GameOptions,
) -> Result<(), String> {
    content::options::write(&game_dir(&state.paths, &id), &options)
}

/// 存档详情（世界名、种子、游戏模式、难度）。
#[tauri::command]
fn level_info(
    state: State<'_, AppState>,
    id: String,
    world: String,
) -> Option<content::leveldat::LevelInfo> {
    content::leveldat::read(&game_dir(&state.paths, &id).join("saves").join(&world))
}

/// 单个模组的详情（描述、作者、依赖）。
#[tauri::command]
fn mod_detail(
    state: State<'_, AppState>,
    id: String,
    file: String,
) -> Option<content::modmeta::ModDetail> {
    content::modmeta::detail(&game_dir(&state.paths, &id), &file)
}
