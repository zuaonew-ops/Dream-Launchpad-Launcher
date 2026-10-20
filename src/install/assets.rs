//! 资源文件（assets/objects）下载。

use crate::net::{download, Job, Progress};
use crate::paths::Paths;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

const CONCURRENCY: usize = 32;
const OBJECT_BASE: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
pub struct AssetObject {
    pub hash: String,
}

pub fn load_index(path: &Path) -> Result<AssetIndex, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("读取资源索引 {} 失败: {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("解析资源索引失败: {e}"))
}

/// 下载索引里所有缺失的对象。
pub async fn download_objects(
    client: &Client,
    paths: &Paths,
    index: &AssetIndex,
    progress: &Progress,
) -> Result<(), String> {
    let jobs: Vec<Job> = index
        .objects
        .values()
        .map(|obj| {
            let prefix = &obj.hash[0..2];
            Job::new(
                format!("{OBJECT_BASE}/{prefix}/{}", obj.hash),
                paths.objects().join(prefix).join(&obj.hash),
                Some(obj.hash.clone()),
            )
        })
        .collect();

    download::run(client, jobs, CONCURRENCY, "下载资源文件", progress).await
}
