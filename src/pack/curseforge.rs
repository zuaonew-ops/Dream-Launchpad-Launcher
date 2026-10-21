//! CurseForge 整合包 zip：清单只给 projectID/fileID，得用官方 API 换直链。

use super::PackPreview;
use crate::net::Job;
use reqwest::Client;
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};

const API: &str = "https://api.curseforge.com/v1";
const USER_AGENT: &str = "DreamLaunchpad/0.26 (+https://github.com/)";

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub name: String,
    #[serde(default)]
    pub version: String,
    pub minecraft: Mc,
    #[serde(default)]
    pub files: Vec<FileRef>,
    #[serde(default = "default_overrides")]
    pub overrides: String,
}

#[derive(Debug, Deserialize)]
pub struct Mc {
    pub version: String,
    #[serde(default, rename = "modLoaders")]
    pub mod_loaders: Vec<ModLoader>,
}

#[derive(Debug, Deserialize)]
pub struct ModLoader {
    pub id: String,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Deserialize)]
pub struct FileRef {
    #[serde(rename = "projectID")]
    pub project_id: u64,
    #[serde(rename = "fileID")]
    pub file_id: u64,
    #[serde(default)]
    pub required: bool,
}

fn default_overrides() -> String {
    "overrides".into()
}

pub fn read_manifest(zip: &Path) -> Result<Manifest, String> {
    let file = std::fs::File::open(zip).map_err(|e| format!("打开 {} 失败: {e}", zip.display()))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("解析 {} 失败: {e}", zip.display()))?;
    let mut entry = archive
        .by_name("manifest.json")
        .map_err(|_| "不是 CurseForge 整合包：没有 manifest.json".to_string())?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|e| format!("读取清单失败: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("解析 manifest.json 失败: {e}"))
}

pub fn preview(zip: &Path) -> Result<PackPreview, String> {
    let m = read_manifest(zip)?;
    let loader = m
        .minecraft
        .mod_loaders
        .iter()
        .find(|l| l.primary)
        .or_else(|| m.minecraft.mod_loaders.first())
        .map(|l| split_loader(&l.id))
        .transpose()?;
    Ok(PackPreview {
        format: "curseforge".into(),
        name: m.name.clone(),
        version: m.version.clone(),
        mc: m.minecraft.version.clone(),
        loader,
        files: m.files.iter().filter(|f| f.required).count(),
        // CF 清单不给大小，下载前不知道总量
        bytes: 0,
        overrides: count_overrides(zip, &m.overrides).unwrap_or(0),
        sample: m
            .files
            .iter()
            .take(12)
            .map(|f| format!("project {} / file {}", f.project_id, f.file_id))
            .collect(),
    })
}

/// CurseForge 的加载器 id 形如 `fabric-0.15.0` / `forge-47.2.0` / `neoforge-21.1.72`。
pub fn split_loader(id: &str) -> Result<(String, String), String> {
    let (name, version) = id
        .split_once('-')
        .ok_or_else(|| format!("不认识的加载器 id: {id}"))?;
    let key = match name {
        "fabric" => "fabric-loader",
        "quilt" => "quilt-loader",
        "forge" => "forge",
        "neoforge" => "neoforge",
        other => return Err(format!("不认识的加载器: {other}")),
    };
    Ok((key.to_string(), version.to_string()))
}

fn count_overrides(zip: &Path, dir: &str) -> Result<usize, String> {
    let file = std::fs::File::open(zip).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let prefix = format!("{}/", dir.trim_end_matches('/'));
    let mut count = 0;
    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index(i) else {
            continue;
        };
        let name = entry.name().replace('\\', "/");
        if name.starts_with(&prefix) && !entry.is_dir() {
            count += 1;
        }
    }
    Ok(count)
}

/// CF 的文件得先拿 key 换直链（`POST /v1/mods/files`，一次最多 1000 个）。
pub async fn jobs(
    client: &Client,
    key: &str,
    zip: &Path,
    dir: &Path,
) -> Result<Vec<Job>, String> {
    if key.trim().is_empty() {
        return Err(
            "CurseForge 整合包需要 API key：到 console.curseforge.com 免费申请一个，\
             填到设置页的「CurseForge API key」里"
                .into(),
        );
    }
    let manifest = read_manifest(zip)?;
    let ids: Vec<u64> = manifest
        .files
        .iter()
        .filter(|f| f.required)
        .map(|f| f.file_id)
        .collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for chunk in ids.chunks(1000) {
        let response = client
            .post(format!("{API}/mods/files"))
            .header("x-api-key", key)
            .header("User-Agent", USER_AGENT)
            .json(&serde_json::json!({ "fileIds": chunk }))
            .send()
            .await
            .map_err(|e| format!("请求 CurseForge 失败: {e}"))?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("CurseForge 返回 {status}: {}", text.chars().take(200).collect::<String>()));
        }
        let parsed: FilesResponse =
            serde_json::from_str(&text).map_err(|e| format!("解析 CurseForge 响应失败: {e}"))?;
        for file in parsed.data {
            let Some(url) = file.download_url.clone().or_else(|| fallback_url(&file)) else {
                return Err(format!(
                    "CurseForge 没给 {} 的下载地址（作者禁用了第三方分发）",
                    file.file_name
                ));
            };
            let sha1 = file
                .hashes
                .iter()
                .find(|h| h.algo == 1)
                .map(|h| h.value.clone());
            out.push(
                Job::mirrored(url, dir.join(&file.file_name), sha1, None)
                    .with_size(file.file_length),
            );
        }
    }
    Ok(out)
}

#[derive(Debug, Deserialize)]
struct FilesResponse {
    #[serde(default)]
    data: Vec<FileInfo>,
}

#[derive(Debug, Deserialize)]
struct FileInfo {
    #[serde(default, rename = "fileName")]
    file_name: String,
    #[serde(default, rename = "downloadUrl")]
    download_url: Option<String>,
    #[serde(default, rename = "fileLength")]
    file_length: u64,
    #[serde(default)]
    hashes: Vec<Hash>,
    #[serde(default)]
    id: u64,
}

#[derive(Debug, Deserialize)]
struct Hash {
    /// 1 = SHA1，2 = MD5（CurseForge 的约定）
    #[serde(default)]
    algo: u32,
    #[serde(default)]
    value: String,
}

/// 作者没开 API 分发时，forgecdn 的直链可以按 fileId 拼出来：
/// `files/<id/1000>/<id%1000>/<文件名>`（社区通用做法，官方没写进文档）。
fn fallback_url(file: &FileInfo) -> Option<String> {
    if file.id == 0 || file.file_name.is_empty() {
        return None;
    }
    Some(format!(
        "https://mediafilez.forgecdn.net/files/{}/{}/{}",
        file.id / 1000,
        file.id % 1000,
        file.file_name
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_loader_ids() {
        assert_eq!(
            split_loader("fabric-0.15.0").unwrap(),
            ("fabric-loader".to_string(), "0.15.0".to_string())
        );
        assert_eq!(
            split_loader("neoforge-21.1.72").unwrap(),
            ("neoforge".to_string(), "21.1.72".to_string())
        );
        assert!(split_loader("weird").is_err());
    }
}
