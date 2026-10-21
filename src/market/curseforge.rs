//! CurseForge 官方 API（需要免费的 API key）。
//!
//! 申请：https://console.curseforge.com/#/api-keys
//! 没配 key 时这里会返回一句可读的错误，而不是静默失败。

use super::curseforge_class;
use super::types::{FileChoice, Hit, ProjectDetail, SearchPage, VersionEntry};
use crate::market::zh as market_zh;
use reqwest::Client;

const API: &str = "https://api.curseforge.com/v1";
const GAME_ID: u32 = 432; // Minecraft

pub(crate) const USER_AGENT: &str =
    "DreamLaunchpad/0.18 (Minecraft launcher; +https://github.com/zuaonew-ops/Dream-Launchpad-Launcher)";

/// 加载器名 → CurseForge 的 modLoaderType
fn loader_type(loader: &str) -> Option<u32> {
    match loader {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

fn game_version(value: Option<&str>) -> Option<String> {
    value.filter(|v| !v.is_empty()).map(str::to_string)
}

fn check_key(key: &str) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err(
            "CurseForge 需要 API key：到 console.curseforge.com 免费申请一个，\
             填到设置页的「CurseForge API key」里"
                .into(),
        );
    }
    Ok(())
}

pub async fn search(
    client: &Client,
    key: &str,
    query: &str,
    kind: &str,
    mc: Option<&str>,
    loader: Option<&str>,
    offset: u64,
    limit: u64,
) -> Result<SearchPage, String> {
    check_key(key)?;

    let mut request = client
        .get(format!("{API}/mods/search"))
        .header("x-api-key", key)
        .header("User-Agent", USER_AGENT)
        .query(&[
            ("gameId", GAME_ID.to_string()),
            ("classId", curseforge_class(kind).to_string()),
            ("index", offset.to_string()),
            ("pageSize", limit.clamp(1, 50).to_string()),
            // 空关键词时按热门排
            (
                "sortField",
                if query.trim().is_empty() { "6" } else { "2" }.to_string(),
            ),
            ("sortOrder", "desc".to_string()),
        ]);

    if !query.trim().is_empty() {
        request = request.query(&[("searchFilter", market_zh::translate_query(query))]);
    }
    if let Some(mc) = game_version(mc) {
        request = request.query(&[("gameVersion", mc)]);
    }
    if let Some(loader_type) = loader.and_then(loader_type) {
        request = request.query(&[("modLoaderType", loader_type.to_string())]);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("请求 CurseForge 失败: {e}"))?;

    if response.status().as_u16() == 403 {
        return Err("CurseForge 拒绝了请求：API key 无效或已过期".into());
    }
    if !response.status().is_success() {
        return Err(format!("CurseForge 返回 HTTP {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析 CurseForge 响应失败: {e}"))?;

    let hits = body
        .get("data")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    let slug = item
                        .get("slug")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let title = item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    Hit {
                        title_zh: market_zh::title_zh(&slug, &title),
                        id: item
                            .get("id")
                            .and_then(|v| v.as_u64())
                            .map(|id| id.to_string())
                            .unwrap_or_default(),
                        slug,
                        title,
                        description: item
                            .get("summary")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        downloads: item.get("downloadCount").and_then(|v| v.as_u64()).unwrap_or(0),
                        icon: item
                            .get("logo")
                            .and_then(|v| v.get("thumbnailUrl"))
                            .and_then(|v| v.as_str())
                            .map(str::to_string),
                        source: "curseforge".to_string(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(SearchPage {
        hits,
        total: body
            .get("pagination")
            .and_then(|p| p.get("totalCount"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        offset,
    })
}

pub async fn versions(
    client: &Client,
    key: &str,
    project: &str,
    mc: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<VersionEntry>, String> {
    check_key(key)?;

    let mut request = client
        .get(format!("{API}/mods/{project}/files"))
        .header("x-api-key", key)
        .header("User-Agent", USER_AGENT)
        .query(&[
            ("pageSize", "50".to_string()),
            ("sortOrder", "desc".to_string()),
        ]);
    if let Some(mc) = game_version(mc) {
        request = request.query(&[("gameVersion", mc)]);
    }
    if let Some(loader_type) = loader.and_then(loader_type) {
        request = request.query(&[("modLoaderType", loader_type.to_string())]);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("请求 CurseForge 文件列表失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("CurseForge 返回 HTTP {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析文件列表失败: {e}"))?;

    // CurseForge 按「文件」组织，这里把它折叠成版本条目：每个文件一版
    let entries = body
        .get("data")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    let file_id = item
                        .get("id")
                        .and_then(|v| v.as_u64())
                        .map(|id| id.to_string())
                        .unwrap_or_default();
                    let filename = item
                        .get("fileName")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let display = item
                        .get("displayName")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&filename)
                        .to_string();
                    let game_versions = item
                        .get("gameVersions")
                        .and_then(|v| v.as_array())
                        .map(|list| {
                            list.iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let loaders = game_versions
                        .iter()
                        .filter(|v| {
                            matches!(
                                v.as_str(),
                                "Forge" | "Fabric" | "Quilt" | "NeoForge"
                            )
                        })
                        .map(|v| v.to_lowercase())
                        .collect::<Vec<_>>();
                    let mc_versions = game_versions
                        .iter()
                        .filter(|v| v.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false))
                        .cloned()
                        .collect::<Vec<_>>();
                    let url = item
                        .get("downloadUrl")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let date = item
                        .get("fileDate")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();

                    VersionEntry {
                        version_id: file_id.clone(),
                        name: display.clone(),
                        version_number: display,
                        downloads: item
                            .get("downloadCount")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
                        game_versions: mc_versions.clone(),
                        loaders: loaders.clone(),
                        date: date.clone(),
                        files: vec![FileChoice {
                            version_id: file_id,
                            name: filename.clone(),
                            version_number: filename.clone(),
                            filename,
                            download_url: url,
                            downloads: item
                                .get("downloadCount")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0),
                            game_versions: mc_versions,
                            loaders,
                            size: item
                                .get("fileLength")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0),
                            date,
                        }],
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(entries)
}

pub async fn detail(client: &Client, key: &str, project: &str) -> Result<ProjectDetail, String> {
    check_key(key)?;

    let response = client
        .get(format!("{API}/mods/{project}"))
        .header("x-api-key", key)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("请求 CurseForge 详情失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("CurseForge 返回 HTTP {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析 CurseForge 详情失败: {e}"))?;

    let data = body.get("data").cloned().unwrap_or(serde_json::Value::Null);
    let slug = data
        .get("slug")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let title = data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    Ok(ProjectDetail {
        title_zh: market_zh::title_zh(&slug, &title),
        id: data
            .get("id")
            .and_then(|v| v.as_u64())
            .map(|id| id.to_string())
            .unwrap_or_default(),
        slug: slug.clone(),
        title,
        description: data
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        body: String::new(),
        icon: data
            .get("logo")
            .and_then(|v| v.get("thumbnailUrl"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        downloads: data
            .get("downloadCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        categories: data
            .get("categories")
            .and_then(|v| v.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|c| c.get("name").and_then(|v| v.as_str()).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        site_url: data
            .get("links")
            .and_then(|v| v.get("websiteUrl"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("https://www.curseforge.com/minecraft/mc-mods/{slug}")),
        source: "curseforge".to_string(),
    })
}
