//! Modrinth `.mrpack`：清单自带直链与哈希，装起来不需要 API key。

use super::PackPreview;
use serde::Deserialize;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Index {
    name: String,
    #[serde(default)]
    version_id: String,
    #[serde(default)]
    files: Vec<FileEntry>,
    #[serde(default)]
    dependencies: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct FileEntry {
    pub path: String,
    #[serde(default)]
    pub hashes: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub downloads: Vec<String>,
    #[serde(default)]
    pub file_size: u64,
    #[serde(default)]
    pub env: Option<Env>,
}

#[derive(Debug, Deserialize)]
pub struct Env {
    #[serde(default)]
    pub client: Option<String>,
}

impl FileEntry {
    /// 服务端专用文件不装（`env.client == "unsupported"`）
    pub fn wanted_on_client(&self) -> bool {
        self.env
            .as_ref()
            .and_then(|e| e.client.as_deref())
            .map(|c| c != "unsupported")
            .unwrap_or(true)
    }

    pub fn sha1(&self) -> Option<String> {
        self.hashes.get("sha1").cloned()
    }

    pub fn url(&self) -> Option<&str> {
        self.downloads.first().map(|s| s.as_str())
    }
}

/// 读清单（zip 里的 `modrinth.index.json`）。
pub fn read_index(zip: &Path) -> Result<Index, String> {
    let file = std::fs::File::open(zip).map_err(|e| format!("打开 {} 失败: {e}", zip.display()))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("解析 {} 失败: {e}", zip.display()))?;
    let mut entry = archive
        .by_name("modrinth.index.json")
        .map_err(|_| "不是 .mrpack：压缩包里没有 modrinth.index.json".to_string())?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|e| format!("读取清单失败: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("解析 modrinth.index.json 失败: {e}"))
}

pub fn is_mrpack(zip: &Path) -> bool {
    read_index(zip).is_ok()
}

/// 预览：给界面看的摘要。
pub fn preview(zip: &Path) -> Result<PackPreview, String> {
    let index = read_index(zip)?;
    let wanted: Vec<&FileEntry> = index.files.iter().filter(|f| f.wanted_on_client()).collect();
    let loader = loader_of(&index.dependencies);
    Ok(PackPreview {
        format: "mrpack".into(),
        name: index.name.clone(),
        version: index.version_id.clone(),
        mc: index.dependencies.get("minecraft").cloned().unwrap_or_default(),
        loader,
        files: wanted.len(),
        bytes: wanted.iter().map(|f| f.file_size).sum(),
        overrides: count_overrides(zip).unwrap_or(0),
        sample: wanted.iter().take(12).map(|f| f.path.clone()).collect(),
    })
}

/// 依赖里的加载器 → (名字, 版本)。键名是 `fabric-loader` / `forge` / `neoforge` / `quilt-loader`。
pub fn loader_of(deps: &std::collections::HashMap<String, String>) -> Option<(String, String)> {
    for key in ["fabric-loader", "quilt-loader", "neoforge", "forge"] {
        if let Some(version) = deps.get(key) {
            return Some((key.to_string(), version.clone()));
        }
    }
    None
}

/// overrides 里的文件数（安装时逐个解压）。
pub fn count_overrides(zip: &Path) -> Result<usize, String> {
    let file = std::fs::File::open(zip).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut count = 0;
    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index(i) else {
            continue;
        };
        let name = entry.name().replace('\\', "/");
        if name.starts_with("overrides/") || name.starts_with("client-overrides/") {
            if !entry.is_dir() {
                count += 1;
            }
        }
    }
    Ok(count)
}

pub(super) fn index_files(zip: &Path) -> Result<Vec<FileEntry>, String> {
    Ok(read_index(zip)?.files)
}

pub(super) fn index_name(zip: &Path) -> Result<String, String> {
    Ok(read_index(zip)?.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_only_files_are_skipped() {
        let f = FileEntry {
            path: "mods/a.jar".into(),
            hashes: Default::default(),
            downloads: vec!["https://example.com/a.jar".into()],
            file_size: 1,
            env: Some(Env {
                client: Some("unsupported".into()),
            }),
        };
        assert!(!f.wanted_on_client());
        let g = FileEntry {
            env: Some(Env {
                client: Some("required".into()),
            }),
            ..FileEntry {
                path: "mods/b.jar".into(),
                hashes: Default::default(),
                downloads: vec![],
                file_size: 1,
                env: None,
            }
        };
        assert!(g.wanted_on_client());
    }

    #[test]
    fn picks_loader_from_dependencies() {
        let mut deps = std::collections::HashMap::new();
        deps.insert("minecraft".to_string(), "1.20.1".to_string());
        deps.insert("fabric-loader".to_string(), "0.15.0".to_string());
        assert_eq!(
            loader_of(&deps),
            Some(("fabric-loader".to_string(), "0.15.0".to_string()))
        );
    }
}
