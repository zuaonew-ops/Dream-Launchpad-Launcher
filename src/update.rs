//! 启动器自身更新检查：查 GitHub Releases 的最新 tag，和本机版本比。
//!
//! 不做自动下载替换（要处理 exe 被占用、签名、回滚），只提示 + 给下载页链接。

use reqwest::Client;
use serde::{Deserialize, Serialize};

const RELEASES: &str = "https://api.github.com/repos/zuaonew-ops/Dream-Launchpad-Launcher/releases/latest";

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    /// 当前版本（编译时打进 binary 的）
    pub current: String,
    /// 远端最新 tag（去掉开头的 v）
    pub latest: String,
    pub url: String,
    /// 有没有新版本
    pub newer: bool,
    pub notes: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    #[serde(default)]
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    body: String,
}

/// 查最新发布版；拿不到（没发过版 / 没网）时返回 None。
pub async fn check(client: &Client, current: &str) -> Result<Option<UpdateInfo>, String> {
    let response = client
        .get(RELEASES)
        .header("User-Agent", "DreamLaunchpad")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("请求 GitHub 失败: {e}"))?;
    // 仓库还没发过 Release 时 GitHub 给 404，这不算错误
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "GitHub 返回 {status}: {}",
            text.chars().take(160).collect::<String>()
        ));
    }
    let release: Release =
        serde_json::from_str(&text).map_err(|e| format!("解析发布信息失败: {e}"))?;
    let latest = release.tag_name.trim_start_matches('v').to_string();
    if latest.is_empty() {
        return Ok(None);
    }
    Ok(Some(UpdateInfo {
        current: current.to_string(),
        newer: is_newer(&latest, current),
        latest,
        url: release.html_url,
        notes: release.body.chars().take(2000).collect(),
    }))
}

/// 版本号比较，规则和模组那边一致：按数字段比。
pub fn is_newer(latest: &str, current: &str) -> bool {
    let a: Vec<u64> = latest
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    let b: Vec<u64> = current
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    for i in 0..a.len().max(b.len()) {
        let x = *a.get(i).unwrap_or(&0);
        let y = *b.get(i).unwrap_or(&0);
        if x != y {
            return x > y;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_launcher_versions() {
        assert!(is_newer("0.34.0", "0.33.0"));
        assert!(is_newer("0.33.1", "0.33.0"));
        assert!(!is_newer("0.33.0", "0.33.0"));
        assert!(!is_newer("0.32.9", "0.33.0"));
    }
}
