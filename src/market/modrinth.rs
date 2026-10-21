//! Modrinth 官方 API（公开、免 key）。
//!
//! 文档：https://docs.modrinth.com/api/

use super::project_type;
use super::types::{FileChoice, Hit, ProjectDetail, SearchPage, VersionEntry};
use crate::market_zh;
use reqwest::Client;

const API: &str = "https://api.modrinth.com/v2";
pub(crate) const USER_AGENT: &str =
    "DreamLaunchpad/0.18 (Minecraft launcher; +https://github.com/zuaonew-ops/Dream-Launchpad-Launcher)";

fn strings(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub async fn search(
    client: &Client,
    query: &str,
    kind: &str,
    mc: Option<&str>,
    loader: Option<&str>,
    category: Option<&str>,
    offset: u64,
    limit: u64,
) -> Result<SearchPage, String> {
    let translated = market_zh::translate_query(query);
    let mut facets = vec![format!("[\"project_type:{}\"]", project_type(kind))];
    if let Some(mc) = mc.filter(|v| !v.is_empty()) {
        facets.push(format!("[\"versions:{mc}\"]"));
    }
    if let Some(loader) = loader.filter(|v| !v.is_empty() && *v != "vanilla") {
        facets.push(format!("[\"categories:{loader}\"]"));
    }
    if let Some(category) = category.filter(|v| !v.is_empty() && *v != "all") {
        facets.push(format!("[\"categories:{category}\"]"));
    }

    // 没有关键词时按下载量排，打开就有一屏热门
    let index = if translated.trim().is_empty() {
        "downloads"
    } else {
        "relevance"
    };

    let response = client
        .get(format!("{API}/search"))
        .query(&[
            ("query", translated),
            ("limit", limit.clamp(1, 50).to_string()),
            ("offset", offset.to_string()),
            ("facets", format!("[{}]", facets.join(","))),
            ("index", index.to_string()),
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
                .map(|item| {
                    let slug = item
                        .get("slug")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let title = item
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    Hit {
                        title_zh: market_zh::title_zh(&slug, &title),
                        id: item
                            .get("project_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&slug)
                            .to_string(),
                        slug,
                        title,
                        description: item
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        downloads: item.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
                        icon: item
                            .get("icon_url")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(str::to_string),
                        source: "modrinth".to_string(),
                    }
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

pub async fn versions(
    client: &Client,
    project: &str,
    mc: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<VersionEntry>, String> {
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

    Ok(body.iter().map(version_entry).collect())
}

fn version_entry(item: &serde_json::Value) -> VersionEntry {
    let version_id = item
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let name = item
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let version_number = item
        .get("version_number")
        .and_then(|v| v.as_str())
        .unwrap_or(name);
    let game_versions = strings(item.get("game_versions"));
    let loaders = strings(item.get("loaders"));
    let date = item
        .get("date_published")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    let files = item
        .get("files")
        .and_then(|v| v.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|file| {
                    let url = file.get("url")?.as_str()?.to_string();
                    Some(FileChoice {
                        version_id: version_id.to_string(),
                        name: name.to_string(),
                        version_number: version_number.to_string(),
                        filename: file
                            .get("filename")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        download_url: url,
                        downloads: file.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
                        game_versions: game_versions.clone(),
                        loaders: loaders.clone(),
                        size: file.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
                        date: date.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    VersionEntry {
        version_id: version_id.to_string(),
        name: name.to_string(),
        version_number: version_number.to_string(),
        downloads: item.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
        game_versions,
        loaders,
        date: date.to_string(),
        files,
    }
}

pub async fn detail(client: &Client, project: &str) -> Result<ProjectDetail, String> {
    let response = client
        .get(format!("{API}/project/{project}"))
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("请求项目详情失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Modrinth 返回 HTTP {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析项目详情失败: {e}"))?;

    let slug = body
        .get("slug")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let title = body
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    Ok(ProjectDetail {
        title_zh: market_zh::title_zh(&slug, &title),
        id: body
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        slug: slug.clone(),
        title,
        description: body
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        body: body
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        icon: body
            .get("icon_url")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        downloads: body.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0),
        categories: strings(body.get("categories")),
        site_url: format!("https://modrinth.com/project/{slug}"),
        source: "modrinth".to_string(),
    })
}
