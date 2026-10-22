//! 模组更新检查：按文件哈希问 Modrinth「这是哪个项目」，再看有没有新版本。
//!
//! 为什么不按文件名猜：同一个 mod 的 jar 名字五花八门，但 **SHA1 是唯一的**。
//! Modrinth 的 `POST /v2/version_files` 支持一次传最多 1000 个哈希，
//! 返回每个哈希对应的项目与版本 —— 比逐个搜索准得多。

use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::modrinth::USER_AGENT;

const API: &str = "https://api.modrinth.com/v2";

/// 一个装好的 mod 在 Modrinth 上的身份。
#[derive(Debug, Clone, Serialize)]
pub struct ModIdentity {
    pub file_name: String,
    pub sha1: String,
    pub project_id: String,
    pub project_title: String,
    pub project_slug: String,
    /// 当前装着的版本号
    pub version_number: String,
}

/// 可更新的 mod。
#[derive(Debug, Clone, Serialize)]
pub struct ModUpdate {
    pub file_name: String,
    pub project_id: String,
    pub project_slug: String,
    pub title: String,
    pub current: String,
    pub latest: String,
    pub download_url: String,
    pub latest_file: String,
}

#[derive(Debug, Deserialize)]
struct VersionFileInfo {
    #[serde(default)]
    project_id: String,
    #[serde(default)]
    version_number: String,
}

/// 批量解析哈希 → 项目身份。认不出的（不在 Modrinth 上的）不会出现在结果里。
pub async fn identify(
    client: &Client,
    files: &[(String, String)],
) -> Result<Vec<ModIdentity>, String> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    // 接口一次最多 1000 个，够用；保险起来还是分批
    let mut out = Vec::new();
    for chunk in files.chunks(500) {
        let hashes: Vec<&str> = chunk.iter().map(|(_, sha1)| sha1.as_str()).collect();
        let response = client
            .post(format!("{API}/version_files"))
            .header("User-Agent", USER_AGENT)
            .json(&serde_json::json!({ "hashes": hashes, "algorithm": "sha1" }))
            .send()
            .await
            .map_err(|e| format!("请求 Modrinth 失败: {e}"))?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!(
                "Modrinth 返回 {status}: {}",
                text.chars().take(200).collect::<String>()
            ));
        }
        let map: std::collections::HashMap<String, VersionFileInfo> =
            serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {e}"))?;

        // 项目详情（名字）单独查，批量接口不给
        let projects: Vec<String> = map
            .values()
            .map(|v| v.project_id.clone())
            .filter(|id| !id.is_empty())
            .collect();
        let titles = project_titles(client, &projects).await;

        for (file_name, sha1) in chunk {
            let Some(info) = map.get(sha1) else {
                continue;
            };
            if info.project_id.is_empty() {
                continue;
            }
            let (title, slug) = titles
                .get(&info.project_id)
                .cloned()
                .unwrap_or_else(|| (info.project_id.clone(), info.project_id.clone()));
            out.push(ModIdentity {
                file_name: file_name.clone(),
                sha1: sha1.clone(),
                project_id: info.project_id.clone(),
                project_title: title,
                project_slug: slug,
                version_number: info.version_number.clone(),
            });
        }
    }
    Ok(out)
}

/// 项目 id → (标题, slug)。一次最多查 100 个。
async fn project_titles(
    client: &Client,
    ids: &[String],
) -> std::collections::HashMap<String, (String, String)> {
    let mut out = std::collections::HashMap::new();
    if ids.is_empty() {
        return out;
    }
    let mut unique: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    unique.sort_unstable();
    unique.dedup();
    for chunk in unique.chunks(100) {
        let list = chunk
            .iter()
            .map(|id| format!("\"{id}\""))
            .collect::<Vec<_>>()
            .join(",");
        let url = format!("{API}/projects?ids=[{list}]");
        let Ok(response) = client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
        else {
            continue;
        };
        let Ok(items) = response.json::<Vec<ProjectBrief>>().await else {
            continue;
        };
        for item in items {
            out.insert(item.id.clone(), (item.title, item.slug));
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct ProjectBrief {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    slug: String,
}

/// 查某个项目在指定《我的世界》版本 / 加载器下的最新文件。
pub async fn latest(
    client: &Client,
    project_id: &str,
    mc: &str,
    loader: &str,
) -> Result<Option<(String, String, String)>, String> {
    let mut url = format!("{API}/project/{project_id}/version");
    let mut query: Vec<String> = Vec::new();
    if !mc.is_empty() {
        query.push(format!("game_versions=[\"{mc}\"]"));
    }
    if !loader.is_empty() && loader != "vanilla" {
        query.push(format!("loaders=[\"{loader}\"]"));
    }
    if !query.is_empty() {
        url.push('?');
        url.push_str(&query.join("&"));
    }
    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("请求 Modrinth 失败: {e}"))?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "Modrinth 返回 {status}: {}",
            text.chars().take(200).collect::<String>()
        ));
    }
    let versions: Vec<VersionInfo> =
        serde_json::from_str(&text).map_err(|e| format!("解析版本列表失败: {e}"))?;
    // 接口按发布时间倒序，第一个就是最新的
    for version in versions {
        if version.files.is_empty() {
            continue;
        }
        let file = version
            .files
            .iter()
            .find(|f| f.primary)
            .or_else(|| version.files.first());
        if let Some(file) = file {
            return Ok(Some((
                version.version_number,
                file.url.clone(),
                file.filename.clone(),
            )));
        }
    }
    Ok(None)
}

#[derive(Debug, Deserialize)]
struct VersionInfo {
    #[serde(default)]
    version_number: String,
    #[serde(default)]
    files: Vec<VersionFile>,
}

#[derive(Debug, Deserialize)]
struct VersionFile {
    #[serde(default)]
    url: String,
    #[serde(default)]
    filename: String,
    #[serde(default)]
    primary: bool,
}

/// 版本号比较：只看前几段数字，`0.5.8` vs `0.5.10` 要判对。
pub fn is_newer(latest: &str, current: &str) -> bool {
    let a = numbers(latest);
    let b = numbers(current);
    for i in 0..a.len().max(b.len()) {
        let x = *a.get(i).unwrap_or(&0);
        let y = *b.get(i).unwrap_or(&0);
        if x != y {
            return x > y;
        }
    }
    latest != current
}

/// 从版本串里抠出数字段（`0.5.8+1.20.1` → [0,5,8,1,20,1]）。
fn numbers(text: &str) -> Vec<u64> {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("0.5.10", "0.5.8"));
        assert!(!is_newer("0.5.8", "0.5.10"));
        assert!(is_newer("1.2.0", "1.1.9"));
        assert!(!is_newer("1.2.0", "1.2.0"));
        assert!(is_newer("2.0.0+mc1.21", "1.9.9+mc1.20"));
    }

    #[test]
    fn parses_version_numbers() {
        assert_eq!(numbers("0.5.8+1.20.1"), vec![0, 5, 8, 1, 20, 1]);
        assert_eq!(numbers("1.0"), vec![1, 0]);
        assert!(numbers("").is_empty());
    }
}
