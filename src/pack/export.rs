//! 导出整合包：把一个装好的版本打成 `.mrpack`，能发给别人或自己备份。
//!
//! 用 Modrinth 格式而不是自定义格式：别的启动器也认。文件全部塞进 `overrides/`
//! （`files` 留空），这样不依赖 Modrinth 上的项目 ID，离线也能装。

use crate::content::game_dir;
use crate::minecraft;
use crate::paths::Paths;
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// 导出时带上的目录 / 文件（相对游戏目录）。
/// 不带 saves（存档是隐私）、logs、crash-reports、以及 natives/ 这类生成物。
const INCLUDE_DIRS: &[&str] = &[
    "mods",
    "config",
    "resourcepacks",
    "shaderpacks",
    "defaultconfigs",
    "kubejs",
];
const INCLUDE_FILES: &[&str] = &["options.txt", "servers.dat"];

#[derive(Debug, Clone, Serialize)]
pub struct ExportResult {
    pub path: String,
    pub files: usize,
    pub bytes: u64,
    pub mc: String,
    pub loader: Option<String>,
}

/// 把一个版本导出成 `.mrpack`。`target` 是输出文件路径。
pub fn export(paths: &Paths, id: &str, target: &Path) -> Result<ExportResult, String> {
    let json_path = paths.version_json(id);
    if !json_path.is_file() {
        return Err(format!("版本 {id} 没有版本 JSON，装完再导出"));
    }
    let version = minecraft::load_version(paths, id)?;

    // 继承链底部的才是真正的《我的世界》版本
    let mc = base_mc(paths, &version);
    // 加载器：整合包版本看它自己的记录，普通版本看 id / 继承链，最后按主类兜底
    let loader = loader_from_record(paths, id)
        .or_else(|| loader_of(id))
        .or_else(|| version.inherits_from.as_deref().and_then(loader_of))
        .or_else(|| version.main_class.as_deref().and_then(loader_from_main_class));

    let dir = game_dir(paths, id, true);

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    let file = std::fs::File::create(target)
        .map_err(|e| format!("创建 {} 失败: {e}", target.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // 1) 清单
    let mut dependencies = serde_json::Map::new();
    dependencies.insert("minecraft".into(), serde_json::Value::String(mc.clone()));
    if let Some(loader) = &loader {
        let (key, version) = split_loader(loader);
        if let (Some(key), Some(version)) = (key, version) {
            dependencies.insert(key.into(), serde_json::Value::String(version));
        }
    }
    let index = serde_json::json!({
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": "1.0",
        "name": id,
        "files": [],
        "dependencies": dependencies,
    });
    zip.start_file("modrinth.index.json", options)
        .map_err(|e| format!("写入清单失败: {e}"))?;
    zip.write_all(
        serde_json::to_string_pretty(&index)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )
    .map_err(|e| format!("写入清单失败: {e}"))?;

    // 2) overrides
    let mut files = 0usize;
    let mut bytes = 0u64;
    for name in INCLUDE_DIRS {
        let from = dir.join(name);
        if from.is_dir() {
            add_dir(&mut zip, &from, &format!("overrides/{name}"), options, &mut files, &mut bytes)?;
        }
    }
    for name in INCLUDE_FILES {
        let from = dir.join(name);
        if from.is_file() {
            zip.start_file(format!("overrides/{name}"), options)
                .map_err(|e| format!("写入 {name} 失败: {e}"))?;
            let data = std::fs::read(&from).map_err(|e| format!("读 {name} 失败: {e}"))?;
            zip.write_all(&data)
                .map_err(|e| format!("写 {name} 失败: {e}"))?;
            files += 1;
            bytes += data.len() as u64;
        }
    }
    zip.finish().map_err(|e| format!("收尾失败: {e}"))?;

    let size = std::fs::metadata(target).map(|m| m.len()).unwrap_or(0);
    Ok(ExportResult {
        path: target.to_string_lossy().to_string(),
        files,
        bytes: if bytes > 0 { bytes } else { size },
        mc,
        loader,
    })
}

/// 递归把目录写进 zip，前缀用 `prefix`。
fn add_dir(
    zip: &mut zip::ZipWriter<std::fs::File>,
    from: &Path,
    prefix: &str,
    options: zip::write::SimpleFileOptions,
    files: &mut usize,
    bytes: &mut u64,
) -> Result<(), String> {
    let entries = std::fs::read_dir(from).map_err(|e| format!("读 {} 失败: {e}", from.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let rel = format!("{prefix}/{name}");
        if path.is_dir() {
            add_dir(zip, &path, &rel, options, files, bytes)?;
        } else {
            zip.start_file(&rel, options)
                .map_err(|e| format!("写入 {rel} 失败: {e}"))?;
            let mut f =
                std::fs::File::open(&path).map_err(|e| format!("读 {} 失败: {e}", path.display()))?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)
                .map_err(|e| format!("读 {} 失败: {e}", path.display()))?;
            zip.write_all(&buf).map_err(|e| format!("写 {rel} 失败: {e}"))?;
            *files += 1;
            *bytes += buf.len() as u64;
        }
    }
    Ok(())
}

/// 顺着继承链找到真正的《我的世界》版本号。
fn base_mc(paths: &Paths, version: &minecraft::version::VersionJson) -> String {
    let mut current = version.clone();
    for _ in 0..8 {
        let Some(parent) = current.inherits_from.clone() else {
            return current.id;
        };
        match minecraft::load_version(paths, &parent) {
            Ok(next) => current = next,
            Err(_) => return parent,
        }
    }
    current.id
}

/// 读 `dlp-modpack.json` 里记的加载器（整合包装出来的版本都有这个文件）。
fn loader_from_record(paths: &Paths, id: &str) -> Option<String> {
    let text = std::fs::read_to_string(paths.version_dir(id).join("dlp-modpack.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let pair = value.get("loader")?.as_array()?;
    let key = pair.first()?.as_str()?;
    let version = pair.get(1)?.as_str()?;
    Some(format!("{key}-{version}"))
}

/// 从主类名推断加载器（版本号这时候不知道，用 latest）。
fn loader_from_main_class(main: &str) -> Option<String> {
    let main = main.to_ascii_lowercase();
    if main.contains("fabricmc") {
        return Some("fabric-loader-latest".into());
    }
    if main.contains("quiltmc") {
        return Some("quilt-loader-latest".into());
    }
    if main.contains("bootstraplauncher") || main.contains("neoforge") {
        return Some("neoforge-latest".into());
    }
    if main.contains("forge") {
        return Some("forge-latest".into());
    }
    None
}

/// 版本 id → 加载器（`fabric-loader-0.19.5-26.1` 这种）。
fn loader_of(id: &str) -> Option<String> {
    let lower = id.to_ascii_lowercase();
    for key in ["fabric-loader", "quilt-loader", "neoforge", "forge"] {
        if lower.starts_with(key) {
            return Some(id.to_string());
        }
    }
    None
}

/// `fabric-loader-0.19.5-26.1` → (`fabric-loader`, `0.19.5`)
fn split_loader(id: &str) -> (Option<&'static str>, Option<String>) {
    let lower = id.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("fabric-loader-") {
        return (Some("fabric-loader"), Some(first_version(rest)));
    }
    if let Some(rest) = lower.strip_prefix("quilt-loader-") {
        return (Some("quilt-loader"), Some(first_version(rest)));
    }
    if let Some(rest) = lower.strip_prefix("neoforge-") {
        return (Some("neoforge"), Some(first_version(rest)));
    }
    if let Some(rest) = lower.strip_prefix("forge-") {
        return (Some("forge"), Some(first_version(rest)));
    }
    (None, None)
}

/// `0.19.5-26.1` → `0.19.5`（加载器版本在前，《我的世界》版本在后）
fn first_version(rest: &str) -> String {
    rest.split('-').next().unwrap_or(rest).to_string()
}

/// 导出文件的默认名：`<版本 id>.mrpack`。
pub fn default_target(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.mrpack"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_loader_ids() {
        assert_eq!(split_loader("fabric-loader-0.19.5-26.1").1.unwrap(), "0.19.5");
        assert_eq!(split_loader("neoforge-21.1.72-1.21.1").1.unwrap(), "21.1.72");
        assert_eq!(split_loader("forge-47.2.0-1.20.1").1.unwrap(), "47.2.0");
        assert_eq!(split_loader("26.1").0, None);
    }

    #[test]
    fn detects_loader_prefix() {
        assert!(loader_of("fabric-loader-0.19.5-26.1").is_some());
        assert!(loader_of("26.1").is_none());
    }
}
