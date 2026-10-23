//! 其它整合包格式：MultiMC/Prism、HMCL，以及「把游戏目录整个打进来」的通用 zip。
//!
//! 这三种的共同点：**文件全在包里**，不需要再去网上按清单下载。
//! 所以安装就是「装好原版/加载器，然后把包解开覆盖上去」。
//!
//! 识别的标记：
//! - MultiMC / Prism：`mmc-pack.json`（components 里有 net.minecraft 与加载器）
//! - HMCL：`modpack.json` 或 `manifest.json` 里的 `gameVersion`
//! - 通用：zip 里有 `.minecraft/` 或顶层的 `mods/` / `config/` / `saves/`

use super::PackPreview;
use std::io::Read;
use std::path::Path;

/// 包里文件所在的顶层目录（`""` 表示就在根上）。
#[derive(Debug, Clone)]
pub struct Layout {
    pub format: String,
    pub name: String,
    pub version: String,
    pub mc: String,
    pub loader: Option<(String, String)>,
    /// 要解压到游戏目录的前缀：`""`、`.minecraft/`、`minecraft/`
    pub prefix: String,
    /// 顺带记一下包里有多少文件（预览里显示）
    pub files: usize,
}

impl Layout {
    fn to_preview(&self) -> PackPreview {
        PackPreview {
            format: self.format.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            mc: self.mc.clone(),
            loader: self.loader.clone(),
            files: 0,
            bytes: 0,
            overrides: self.files,
            sample: Vec::new(),
        }
    }
}

/// 识别 MultiMC / Prism / HMCL / 通用 zip；不是这几种就返回 None。
pub fn detect(zip: &Path) -> Option<Layout> {
    let file = std::fs::File::open(zip).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    // 记下 index：Windows 的 Compress-Archive 用反斜杠分隔，by_name("/") 会找不到
    let entries: Vec<(usize, String)> = (0..archive.len())
        .filter_map(|i| {
            archive
                .by_index(i)
                .ok()
                .map(|e| (i, e.name().replace('\\', "/")))
        })
        .filter(|(_, n)| !n.is_empty() && !n.starts_with("__MACOSX/"))
        .collect();
    let names: Vec<String> = entries.iter().map(|(_, n)| n.clone()).collect();

    let has = |name: &str| names.iter().any(|n| n == name || n.ends_with(&format!("/{name}")));

    // MultiMC / Prism
    if let Some((index, path)) = entries.iter().find(|(_, n)| n.ends_with("mmc-pack.json")) {
        if let Ok(text) = read_entry_at(&mut archive, *index) {
            if let Some(layout) = parse_mmc(&text, &names, path) {
                return Some(layout);
            }
        }
    }

    // HMCL / 通用包：modpack.json 或 manifest.json 里有 gameVersion
    for marker in ["modpack.json", "manifest.json"] {
        if let Some((index, _)) = entries.iter().find(|(_, n)| n.ends_with(marker)) {
            if let Ok(text) = read_entry_at(&mut archive, *index) {
                if let Some(layout) = parse_modpack_json(&text, &names) {
                    return Some(layout);
                }
            }
        }
    }

    // 通用：看有没有游戏目录结构
    if let Some(layout) = generic_layout(&names) {
        return Some(layout);
    }
    let _ = has;
    None
}

fn read_entry_at(archive: &mut zip::ZipArchive<std::fs::File>, index: usize) -> Result<String, String> {
    let mut entry = archive
        .by_index(index)
        .map_err(|e| format!("读取第 {index} 个文件失败: {e}"))?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|e| format!("读取第 {index} 个文件失败: {e}"))?;
    // 记事本/VS 生成的 JSON 常带 BOM，serde_json 会当成非法字符
    Ok(text.trim_start_matches('﻿').to_string())
}

#[allow(dead_code)]
fn read_entry(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Result<String, String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|e| format!("读取 {name} 失败: {e}"))?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|e| format!("读取 {name} 失败: {e}"))?;
    // 记事本/VS 生成的 JSON 常带 BOM，serde_json 会当成非法字符
    Ok(text.trim_start_matches('﻿').to_string())
}

/// MultiMC 的 `mmc-pack.json`：components 里躺着游戏版本和加载器版本。
fn parse_mmc(text: &str, names: &[String], pack_path: &str) -> Option<Layout> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let mut mc = String::new();
    let mut loader: Option<(String, String)> = None;
    for component in value.get("components")?.as_array()? {
        let uid = component.get("uid")?.as_str()?;
        let version = component
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        match uid {
            "net.minecraft" => mc = version,
            "net.fabricmc.fabric-loader" => loader = Some(("fabric-loader".into(), version)),
            "org.quiltmc.quilt-loader" => loader = Some(("quilt-loader".into(), version)),
            "net.minecraftforge" => loader = Some(("forge".into(), version)),
            "net.neoforged" => loader = Some(("neoforge".into(), version)),
            _ => {}
        }
    }
    if mc.is_empty() {
        return None;
    }
    // MultiMC 的实例目录名就是包里那一层，游戏文件在 .minecraft/ 或 minecraft/
    let base = pack_path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let prefix = pick_prefix(names, base);
    Some(Layout {
        format: "multimc".into(),
        name: base.rsplit('/').next().unwrap_or("MultiMC 整合包").to_string(),
        version: String::new(),
        mc,
        loader,
        prefix,
        files: names.len(),
    })
}

/// `modpack.json` / `manifest.json`：HMCL 用 `gameVersion`，FTB 用 `minecraft.version`。
fn parse_modpack_json(text: &str, names: &[String]) -> Option<Layout> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    // CurseForge / FTB 那种带 projectID 的交给 curseforge 模块，这里只认自带文件的
    let files_have_url = value
        .get("files")
        .and_then(|v| v.as_array())
        .map(|list| {
            list.iter()
                .any(|f| f.get("url").is_some() || f.get("downloads").is_some())
        })
        .unwrap_or(false);
    if value.get("files").is_some() && !files_have_url {
        return None;
    }

    let mc = value
        .get("gameVersion")
        .and_then(|v| v.as_str())
        .or_else(|| value.pointer("/minecraft/version").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();
    if mc.is_empty() {
        return None;
    }

    // 加载器：HMCL 写 "modLoader": {"type": "fabric", "version": "0.15.0"}
    let loader = value.get("modLoader").and_then(|v| {
        let kind = v.get("type")?.as_str()?;
        let version = v.get("version")?.as_str().unwrap_or("");
        let id = match kind.to_lowercase().as_str() {
            "fabric" => "fabric-loader",
            "quilt" => "quilt-loader",
            "forge" => "forge",
            "neoforge" => "neoforge",
            _ => return None,
        };
        Some((id.to_string(), version.to_string()))
    });

    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("整合包")
        .to_string();
    Some(Layout {
        format: "generic".into(),
        name,
        version: value
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        mc,
        loader,
        prefix: pick_prefix(names, ""),
        files: names.len(),
    })
}

/// 没有任何清单，但结构像游戏目录 —— 当纯覆盖包。
fn generic_layout(names: &[String]) -> Option<Layout> {
    let prefix = pick_prefix(names, "");
    let looks_like_game = names.iter().any(|n| {
        let rest = n.strip_prefix(&prefix).unwrap_or(n);
        rest == "options.txt"
            || rest.starts_with("mods/")
            || rest.starts_with("config/")
            || rest.starts_with("saves/")
            || rest.starts_with("resourcepacks/")
            || rest.starts_with("shaderpacks/")
    });
    if !looks_like_game {
        return None;
    }
    Some(Layout {
        format: "overlay".into(),
        name: "整合包".into(),
        version: String::new(),
        // 没有清单就不知道要哪个游戏版本，让用户自己选
        mc: String::new(),
        loader: None,
        prefix,
        files: names.len(),
    })
}

/// 游戏文件那一层的前缀：`""`、`.minecraft/`、`minecraft/`、`<实例>/.minecraft/`。
fn pick_prefix(names: &[String], base: &str) -> String {
    let base_prefix = if base.is_empty() {
        String::new()
    } else {
        format!("{base}/")
    };
    for candidate in [".minecraft", "minecraft"] {
        let target = format!("{base_prefix}{candidate}/");
        if names.iter().any(|n| n.starts_with(&target)) {
            return target;
        }
    }
    base_prefix
}

pub fn preview(zip: &Path) -> Option<PackPreview> {
    detect(zip).map(|layout| layout.to_preview())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(path: &Path, entries: &[(&str, &str)]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, content) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn reads_multimc_pack() {
        let dir = crate::testdir::temp_dir("dlp-mmc");
        std::fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("实例.zip");
        make_zip(
            &zip_path,
            &[
                (
                    "我的实例/mmc-pack.json",
                    r#"{"formatVersion":1,"components":[
                        {"uid":"net.minecraft","version":"1.20.1"},
                        {"uid":"net.fabricmc.fabric-loader","version":"0.15.0"}]}"#,
                ),
                ("我的实例/.minecraft/mods/a.jar", ""),
                ("我的实例/instance.cfg", "name=我的实例"),
            ],
        );

        let layout = detect(&zip_path).expect("应该认出来");
        assert_eq!(layout.format, "multimc");
        assert_eq!(layout.mc, "1.20.1");
        assert_eq!(
            layout.loader,
            Some(("fabric-loader".to_string(), "0.15.0".to_string()))
        );
        assert_eq!(layout.prefix, "我的实例/.minecraft/");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_hmcl_pack() {
        let dir = crate::testdir::temp_dir("dlp-hmcl");
        std::fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("hmcl.zip");
        make_zip(
            &zip_path,
            &[
                (
                    "modpack.json",
                    r#"{"name":"HMCL 包","version":"1.0","gameVersion":"1.19.2",
                        "modLoader":{"type":"fabric","version":"0.14.0"}}"#,
                ),
                (".minecraft/mods/x.jar", ""),
            ],
        );

        let layout = detect(&zip_path).expect("应该认出来");
        assert_eq!(layout.format, "generic");
        assert_eq!(layout.mc, "1.19.2");
        assert_eq!(layout.name, "HMCL 包");
        assert_eq!(layout.prefix, ".minecraft/");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn treats_plain_game_folder_as_overlay() {
        let dir = crate::testdir::temp_dir("dlp-overlay");
        std::fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("overlay.zip");
        make_zip(&zip_path, &[("mods/a.jar", ""), ("config/b.toml", "")]);

        let layout = detect(&zip_path).expect("应该当成覆盖包");
        assert_eq!(layout.format, "overlay");
        assert_eq!(layout.prefix, "");
        assert!(layout.mc.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ignores_unrelated_zip() {
        let dir = crate::testdir::temp_dir("dlp-notpack");
        std::fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("other.zip");
        make_zip(&zip_path, &[("readme.txt", "hi")]);
        assert!(detect(&zip_path).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
