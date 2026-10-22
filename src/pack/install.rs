//! 把整合包装成一个新版本：下载清单里的文件 → 解压 overrides → 落盘版本 JSON。

use super::{curseforge, mrpack, PackPreview};
use crate::content::game_dir;
use crate::install;
use crate::loader::{self, LoaderKind};
use crate::net::{download, Job, Progress};
use crate::paths::Paths;
use reqwest::Client;
use std::io::Read;
use std::path::{Path, PathBuf};

/// 整合包装出来的版本 id：`<包名>-<版本>`，去掉不适合做目录名的字符。
pub fn version_id(preview: &PackPreview) -> String {
    let raw = if preview.version.is_empty() {
        preview.name.clone()
    } else {
        format!("{}-{}", preview.name, preview.version)
    };
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '\0'
            }
        })
        .filter(|c| *c != '\0')
        .collect();
    let trimmed = cleaned.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "modpack".into()
    } else {
        trimmed
    }
}

/// 识别格式并给预览；不是整合包就报可读错误。
pub fn preview(zip: &Path) -> Result<PackPreview, String> {
    if let Ok(p) = mrpack::preview(zip) {
        return Ok(p);
    }
    if let Ok(p) = curseforge::preview(zip) {
        return Ok(p);
    }
    Err("这个 zip 里既没有 modrinth.index.json 也没有 manifest.json，不是整合包".into())
}

/// 安装整合包。`id` 为空时按包名自动生成。
#[allow(clippy::too_many_arguments)]
pub async fn install(
    client: &Client,
    paths: &Paths,
    zip: &Path,
    id: &str,
    cf_key: &str,
    progress: Option<Progress>,
) -> Result<String, String> {
    let preview = preview(zip)?;
    let id = if id.trim().is_empty() {
        version_id(&preview)
    } else {
        id.to_string()
    };

    // 1) 先按包声明的加载器把版本本体装好（加载器版本会带上原版依赖）
    let target = ensure_base(client, paths, &preview, &id, progress.clone()).await?;

    // 2) 清单里的文件下到游戏目录（整合包版本默认隔离，所以就是版本目录）
    let dir = game_dir(paths, &target, true);
    let jobs = jobs_for(client, cf_key, zip, &preview, &dir).await?;
    let total = jobs.len();
    if total > 0 {
        download::run(client, jobs, 16, "下载整合包文件", &progress.clone().unwrap_or(Progress::none()))
            .await?;
    }

    // 3) overrides 覆盖到游戏目录
    let overrides = unpack_overrides(zip, &dir)?;

    // 4) 版本目录里留个说明，界面能显示整合包信息
    // 记下加载器，导出时不用再从 mainClass 猜
    let loader_json = match &preview.loader {
        Some((key, version)) => format!(
            "[{},{}]",
            serde_json::to_string(key).unwrap_or_else(|_| "\"\"".into()),
            serde_json::to_string(version).unwrap_or_else(|_| "\"\"".into())
        ),
        None => "null".into(),
    };
    let info = format!(
        "{{\"name\":{},\"version\":{},\"files\":{total},\"overrides\":{overrides},\"loader\":{loader_json}}}",
        serde_json::to_string(&preview.name).unwrap_or_else(|_| "\"\"".into()),
        serde_json::to_string(&preview.version).unwrap_or_else(|_| "\"\"".into()),
    );
    let _ = std::fs::write(paths.version_dir(&target).join("dlp-modpack.json"), info);

    Ok(target)
}

/// 按清单依赖装版本本体；原版包直接装原版。
async fn ensure_base(
    client: &Client,
    paths: &Paths,
    preview: &PackPreview,
    id: &str,
    progress: Option<Progress>,
) -> Result<String, String> {
    if preview.mc.is_empty() {
        return Err("整合包没写它依赖哪个《我的世界》版本，装不了".into());
    }
    match &preview.loader {
        Some((name, version)) => {
            let kind = match name.as_str() {
                "fabric-loader" => LoaderKind::Fabric,
                "quilt-loader" => LoaderKind::Quilt,
                "forge" => LoaderKind::Forge,
                "neoforge" => LoaderKind::NeoForge,
                other => return Err(format!("不认识的加载器 {other}")),
            };
            // 加载器装出来的是 <mc>-<loader>-<版本>；整合包要的是自己那份 id，
            // 装完把版本目录复制/改名成 id（JSON 里的 id 也一起改）
            // 导出时推不出具体版本会写 "latest"；Fabric/Quilt 的 meta 认 "stable"
            let version = if version == "latest" { "stable" } else { version.as_str() };
            let produced =
                loader::install_meta(client, paths, &preview.mc, kind, version, progress.clone())
                    .await?;
            adopt_as(paths, &produced, id)?;
            Ok(id.to_string())
        }
        None => {
            install::install(
                client,
                paths,
                &preview.mc,
                progress.clone().unwrap_or(Progress::none()),
            )
            .await?;
            // 原版整合包也复制一份成自己的 id，避免和手动装的原版共用目录
            adopt_as(paths, &preview.mc, id)?;
            Ok(id.to_string())
        }
    }
}

/// 把装好的版本目录另存为整合包自己的 id。
///
/// 为什么要另存：整合包的 mods 是它自己的，和手动装的加载器版本混在一起会互相污染。
fn adopt_as(paths: &Paths, from: &str, to: &str) -> Result<(), String> {
    if from == to {
        return Ok(());
    }
    install::clone::clone_version(paths, from, to)
}

/// 清单 → 下载任务。文件按清单里的相对路径落到游戏目录。
async fn jobs_for(
    client: &Client,
    cf_key: &str,
    zip: &Path,
    preview: &PackPreview,
    dir: &Path,
) -> Result<Vec<Job>, String> {
    let dir: PathBuf = dir.to_path_buf();
    if preview.format == "mrpack" {
        let files = mrpack::index_files(zip)?;
        Ok(files
            .into_iter()
            .filter(|f| f.wanted_on_client())
            .filter_map(|f| {
                let url = f.url()?.to_string();
                let dest = dir.join(&f.path);
                Some(Job::mirrored(url, dest, f.sha1(), None).with_size(f.file_size))
            })
            .collect())
    } else {
        curseforge::jobs(client, cf_key, zip, &dir).await
    }
}

/// 解压 `overrides/` 与 `client-overrides/` 到游戏目录，返回文件数。
pub fn unpack_overrides(zip: &Path, dir: &Path) -> Result<usize, String> {
    let file = std::fs::File::open(zip).map_err(|e| format!("打开 {} 失败: {e}", zip.display()))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("解析 {} 失败: {e}", zip.display()))?;
    let mut count = 0;
    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.name().replace('\\', "/");
        let rel = name
            .strip_prefix("overrides/")
            .or_else(|| name.strip_prefix("client-overrides/"));
        let Some(rel) = rel else { continue };
        if rel.is_empty() {
            continue;
        }
        let out = dir.join(rel);
        if entry.is_dir() {
            let _ = std::fs::create_dir_all(&out);
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|e| format!("读取 {name} 失败: {e}"))?;
        std::fs::write(&out, bytes).map_err(|e| format!("写入 {} 失败: {e}", out.display()))?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_id_is_filesystem_safe() {
        let preview = PackPreview {
            format: "mrpack".into(),
            name: "Fabulously Optimized".into(),
            version: "5.9.0".into(),
            mc: "1.20.1".into(),
            loader: None,
            files: 0,
            bytes: 0,
            overrides: 0,
            sample: vec![],
        };
        assert_eq!(version_id(&preview), "Fabulously-Optimized-5.9.0");
    }

    #[test]
    fn version_id_drops_path_separators() {
        let preview = PackPreview {
            format: "mrpack".into(),
            name: "a/b\\c:d".into(),
            version: String::new(),
            mc: String::new(),
            loader: None,
            files: 0,
            bytes: 0,
            overrides: 0,
            sample: vec![],
        };
        assert_eq!(version_id(&preview), "abcd");
    }
}
