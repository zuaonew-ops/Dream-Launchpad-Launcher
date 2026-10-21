//! 下载安装：把文件铺到版本目录对应的子目录里。
//!
//! - `install`：按项目自动挑「最新且匹配版本/加载器」的文件
//! - `install_file`：用户在详情页点了哪个文件就下哪个

use super::{project_type, target_dir, Source};
use crate::net::{download, Job, Progress};
use reqwest::Client;
use std::path::Path;

const API: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str =
    "DreamLaunchpad/0.18 (Minecraft launcher; +https://github.com/zuaonew-ops/Dream-Launchpad-Launcher)";

/// 把任意 URL 的文件下到 `game_dir/<子目录>/`，返回文件名。
pub async fn install_file(
    client: &Client,
    game_dir: &Path,
    kind: &str,
    download_url: &str,
    filename: &str,
    progress: Option<Progress>,
) -> Result<String, String> {
    if filename.trim().is_empty() {
        return Err("这个文件没有文件名，不能下载".into());
    }
    // 防止路径穿越：只取最后一段
    let safe_name = filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(filename)
        .to_string();

    let dir = game_dir.join(target_dir(kind));
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建 {} 失败: {e}", dir.display()))?;
    let dest = dir.join(&safe_name);

    let job = Job::new(download_url, &dest, None);
    download::run(
        client,
        vec![job],
        1,
        "下载资源",
        &progress.unwrap_or_else(Progress::none),
    )
    .await?;

    Ok(safe_name)
}

/// 只在 Modrinth 上有「按项目自动挑文件」的语义；CurseForge 走 `install_file`。
pub async fn install(
    client: &Client,
    game_dir: &Path,
    source: Source,
    project: &str,
    kind: &str,
    mc: Option<&str>,
    loader: Option<&str>,
    progress: Option<Progress>,
) -> Result<String, String> {
    if source != Source::Modrinth {
        return Err("CurseForge 请到详情页选具体文件下载".into());
    }

    let mut request = client
        .get(format!("{API}/project/{project}/version"))
        .header("User-Agent", USER_AGENT);
    if let Some(mc) = mc.filter(|v| !v.is_empty()) {
        request = request.query(&[("game_versions", format!("[\"{mc}\"]"))]);
    }
    if let Some(loader) = loader.filter(|v| !v.is_empty() && *v != "vanilla") {
        request = request.query(&[("loaders", format!("[\"{loader}\"]"))]);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("请求版本列表失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Modrinth 返回 HTTP {}", response.status()));
    }

    let body: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|e| format!("解析版本列表失败: {e}"))?;

    let file = body
        .first()
        .and_then(|item| item.get("files").and_then(|v| v.as_array()))
        .and_then(|files| {
            files
                .iter()
                .find(|f| f.get("primary").and_then(|p| p.as_bool()).unwrap_or(false))
                .or_else(|| files.first())
        })
        .ok_or_else(|| "这个版本没有匹配的文件".to_string())?;

    let filename = file
        .get("filename")
        .and_then(|v| v.as_str())
        .ok_or("文件缺少 filename")?;
    let download_url = file
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("文件缺少下载地址")?;

    install_file(client, game_dir, kind, download_url, filename, progress).await
}

/// 让编译器知道 `project_type` 这里确实用不到（保留门面里的工具函数引用）
#[allow(dead_code)]
fn _kind_guard(kind: &str) -> &'static str {
    project_type(kind)
}
