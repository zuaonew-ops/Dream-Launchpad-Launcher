//! 读 jar 里的模组元数据：判断是不是同一个 mod 装了两份。
//!
//! 三种格式都有：Fabric 的 `fabric.mod.json`、Forge/NeoForge 的
//! `META-INF/mods.toml`、以及更老的 `mcmod.info`。

use serde::Serialize;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct ModMeta {
    pub file_name: String,
    /// 模组内部 id（`fabric.mod.json` 的 `id` / `mods.toml` 的 `modId`）
    pub id: String,
    pub version: String,
    pub name: String,
}

/// 同一个 mod id 出现在多个文件里 —— 通常会让游戏起不来或行为异常。
#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroup {
    pub id: String,
    pub name: String,
    /// 涉及的 jar 文件名
    pub files: Vec<String>,
}

/// 读一个 jar 的模组信息；不是模组（光影/资源包）时返回 None。
pub fn read_meta(jar: &Path) -> Option<ModMeta> {
    let file_name = jar.file_name()?.to_string_lossy().to_string();
    let file = std::fs::File::open(jar).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;

    // Fabric
    if let Some((id, version, name)) = read_fabric_mod_json(&mut archive) {
        return Some(ModMeta {
            file_name,
            id,
            version,
            name,
        });
    }
    // Forge / NeoForge
    for entry in ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"] {
        if let Some((id, version, name)) = read_mods_toml(&mut archive, entry) {
            return Some(ModMeta {
                file_name,
                id,
                version,
                name,
            });
        }
    }
    // 老版 Forge
    if let Some((id, version, name)) = read_mcmod_info(&mut archive) {
        return Some(ModMeta {
            file_name,
            id,
            version,
            name,
        });
    }
    None
}

/// 列出目录里所有 jar 的元数据（读不出来就跳过）。
pub fn scan(dir: &Path) -> Vec<ModMeta> {
    let Ok(entries) = std::fs::read_dir(dir.join("mods")) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jar") {
            continue;
        }
        if let Some(meta) = read_meta(&path) {
            out.push(meta);
        }
    }
    out
}

/// 找出装了多个版本的 mod。
pub fn duplicates(dir: &Path) -> Vec<DuplicateGroup> {
    let metas = scan(dir);
    let mut groups: Vec<DuplicateGroup> = Vec::new();
    for meta in metas {
        if meta.id.is_empty() {
            continue;
        }
        match groups.iter_mut().find(|g| g.id == meta.id) {
            Some(group) => group.files.push(meta.file_name),
            None => groups.push(DuplicateGroup {
                id: meta.id,
                name: meta.name,
                files: vec![meta.file_name],
            }),
        }
    }
    groups.retain(|g| g.files.len() > 1);
    groups
}

fn read_entry(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    Some(text)
}

fn read_fabric_mod_json(
    archive: &mut zip::ZipArchive<std::fs::File>,
) -> Option<(String, String, String)> {
    let text = read_entry(archive, "fabric.mod.json")?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let id = value.get("id")?.as_str()?.to_string();
    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("?")
        .to_string();
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&id)
        .to_string();
    Some((id, version, name))
}

/// `mods.toml` 里第一个 `[[mods]]` 段的 modId / version / displayName。
fn read_mods_toml(
    archive: &mut zip::ZipArchive<std::fs::File>,
    entry: &str,
) -> Option<(String, String, String)> {
    let text = read_entry(archive, entry)?;
    let id = toml_value(&text, "modId")?;
    let version = toml_value(&text, "version").unwrap_or_else(|| "?".into());
    let name = toml_value(&text, "displayName").unwrap_or_else(|| id.clone());
    Some((id, version, name))
}

/// 从 toml 里抠 `key = "value"`（只认带引号的字符串，够用）。
///
/// 不引 toml 库：这里只需要三个字段，而且 `mods.toml` 里还有
/// `${file.jarVersion}` 这类占位符，正经解析器反而会报错。
fn toml_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some(rest) = line.strip_prefix(key) else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim();
        let value = if let Some(v) = rest.strip_prefix('"') {
            v.split('"').next().unwrap_or("")
        } else if let Some(v) = rest.strip_prefix('\'') {
            v.split('\'').next().unwrap_or("")
        } else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        // ${file.jarVersion} 这种占位符没法在本地求值，当作未知
        let value = value.replace("${file.jarVersion}", "?");
        return Some(value);
    }
    None
}

fn read_mcmod_info(
    archive: &mut zip::ZipArchive<std::fs::File>,
) -> Option<(String, String, String)> {
    let text = read_entry(archive, "mcmod.info")?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let first = value.as_array()?.first()?;
    let id = first.get("modid")?.as_str()?.to_string();
    let version = first
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("?")
        .to_string();
    let name = first
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&id)
        .to_string();
    Some((id, version, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_toml_strings() {
        let text = r#"
modLoader="javafml"
loaderVersion="[47,)"
[[mods]]
modId="sodium"
version="0.5.8"
displayName="Sodium"
"#;
        assert_eq!(toml_value(text, "modId").unwrap(), "sodium");
        assert_eq!(toml_value(text, "version").unwrap(), "0.5.8");
        assert_eq!(toml_value(text, "displayName").unwrap(), "Sodium");
        assert!(toml_value(text, "missing").is_none());
    }

    #[test]
    fn treats_jar_version_placeholder_as_unknown() {
        let text = "[[mods]]\nmodId=\"forge\"\nversion=\"${file.jarVersion}\"\n";
        assert_eq!(toml_value(text, "version").unwrap(), "?");
    }

    #[test]
    fn finds_duplicates_by_mod_id() {
        let dir = crate::testdir::temp_dir("dlp-mods-dup");
        let mods = dir.join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        for (name, id) in [
            ("sodium-0.5.0.jar", "sodium"),
            ("sodium-0.5.13.jar", "sodium"),
            ("lithium.jar", "lithium"),
        ] {
            let path = mods.join(name);
            let file = std::fs::File::create(&path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("fabric.mod.json", zip::write::SimpleFileOptions::default())
                .unwrap();
            use std::io::Write;
            zip.write_all(format!(r#"{{"id":"{id}","version":"1.0","name":"{id}"}}"#).as_bytes())
                .unwrap();
            zip.finish().unwrap();
        }
        let groups = duplicates(&dir);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "sodium");
        assert_eq!(groups[0].files.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
