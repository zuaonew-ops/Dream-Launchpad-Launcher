//! 版本清单（version_manifest_v2.json）。

use reqwest::Client;
use serde::{Deserialize, Serialize};

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
    #[serde(default)]
    pub sha1: Option<String>,
}

impl VersionEntry {
    pub fn is_release(&self) -> bool {
        self.kind == "release"
    }
}

/// 拉取版本清单。
pub async fn fetch(client: &Client) -> Result<VersionManifest, String> {
    client
        .get(MANIFEST_URL)
        .send()
        .await
        .map_err(|e| format!("获取版本清单失败: {e}"))?
        .error_for_status()
        .map_err(|e| format!("获取版本清单失败: {e}"))?
        .json::<VersionManifest>()
        .await
        .map_err(|e| format!("解析版本清单失败: {e}"))
}
