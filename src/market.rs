//! Modrinth 资源搜索与安装（模组 / 材质包 / 光影）。
//!
//! 只用公开 API，不需要 key；安装就是把主文件下到版本目录对应的子目录里。

use crate::net::{download, Job, Progress};
use reqwest::Client;
use serde::Serialize;
use std::path::Path;

const API: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str =
    "DreamLaunchpad/0.9 (Minecraft launcher; +https://github.com/zuaonew-ops/Dream-Launchpad-Launcher)";

#[derive(Serialize, Clone, Debug)]
pub struct Hit {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub downloads: u64,
    pub icon: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct SearchPage {
    pub hits: Vec<Hit>,
    pub total: u64,
    pub offset: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct FileChoice {
    pub version_id: String,
    pub name: String,
    pub filename: String,
    pub downloads: u64,
    pub game_versions: Vec<String>,
}

/// 资源种类 → 游戏目录下的子目录
pub fn target_dir(kind: &str) -> &'static str {
    match kind {
        "resourcepack" => "resourcepacks",
        "shader" => "shaderpacks",
        _ => "mods",
    }
}

fn project_type(kind: &str) -> &'static str {
    match kind {
        "resourcepack" => "resourcepack",
        "shader" => "shader",
        _ => "mod",
    }
}

pub async fn search(
    client: &Client,
    query: &str,
    kind: &str,
    mc: Option<&str>,
    loader: Option<&str>,
    offset: u64,
    limit: u64,
) -> Result<SearchPage, String> {
    let mut facets = vec![format!("[\"project_type:{}\"]", project_type(kind))];
    if let Some(mc) = mc.filter(|v| !v.is_empty()) {
        facets.push(format!("[\"versions:{mc}\"]"));
    }
    if let Some(loader) = loader.filter(|v| !v.is_empty() && *v != "vanilla") {
        facets.push(format!("[\"categories:{loader}\"]"));
    }

    let url = format!("{API}/search");
    let response = client
        .get(&url)
        .query(&[
            ("query", query.to_string()),
            ("limit", limit.clamp(1, 50).to_string()),
            ("offset", offset.to_string()),
            ("facets", format!("[{}]", facets.join(","))),
            ("index", "relevance".to_string()),
        ])
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("请求 Modrinth 失败: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Modrinth 返回 HTTP {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析搜索结果失败: {e}"))?;

    let hits = body
        .get("hits")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .map(|item| Hit {
                    id: item.get("project_id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    slug: item.get("slug").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    title: item.get("title").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    description: item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    downloads: item.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
                    icon: item.get("icon_url").and_then(|v| v.as_str()).map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(SearchPage {
        hits,
        total: body.get("total_hits").and_then(|v| v.as_u64()).unwrap_or(0),
        offset,
    })
}

/// 某项目在「这个游戏版本 + 这个加载器」下可用的文件（新到旧）。
pub async fn files(
    client: &Client,
    project: &str,
    mc: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<FileChoice>, String> {
    let url = format!("{API}/project/{project}/version");
    let mut request = client.get(&url).header("User-Agent", USER_AGENT);
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

    Ok(body
        .iter()
        .map(|item| {
            let files = item.get("files").and_then(|v| v.as_array());
            let primary = files
                .and_then(|list| {
                    list.iter()
                        .find(|f| f.get("primary").and_then(|p| p.as_bool()).unwrap_or(false))
                        .or_else(|| list.first())
                })
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            FileChoice {
                version_id: item.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                name: item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                filename: primary
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                downloads: item.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
                game_versions: item
                    .get("game_versions")
                    .and_then(|v| v.as_array())
                    .map(|list| {
                        list.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        })
        .collect())
}

/// 装最新一个可用文件到 `game_dir/<子目录>/`，返回文件名。
pub async fn install(
    client: &Client,
    game_dir: &Path,
    project: &str,
    kind: &str,
    mc: Option<&str>,
    loader: Option<&str>,
    progress: Option<Progress>,
) -> Result<String, String> {
    let url = format!("{API}/project/{project}/version");
    let mut request = client.get(&url).header("User-Agent", USER_AGENT);
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
    let sha1 = file
        .get("hashes")
        .and_then(|h| h.get("sha1"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let dir = game_dir.join(target_dir(kind));
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建 {} 失败: {e}", dir.display()))?;
    let dest = dir.join(filename);

    let job = Job::new(download_url, &dest, sha1);
    download::run(
        client,
        vec![job],
        1,
        "下载资源",
        &progress.unwrap_or_else(Progress::none),
    )
    .await?;

    Ok(filename.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_map_to_game_dirs() {
        assert_eq!(target_dir("mod"), "mods");
        assert_eq!(target_dir("resourcepack"), "resourcepacks");
        assert_eq!(target_dir("shader"), "shaderpacks");
        assert_eq!(project_type("shader"), "shader");
    }
}
