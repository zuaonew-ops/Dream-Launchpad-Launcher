//! natives 解压（把原生库 jar 展开到 `versions/<id>/natives`）。

use crate::minecraft::{platform, VersionJson};
use crate::net::Progress;
use crate::paths::Paths;
use std::io::Read;
use std::path::{Path, PathBuf};

const EXCLUDES: [&str; 1] = ["META-INF/"];

/// 展开当前平台所有 natives jar。
///
/// 两种格式都要支持：
/// - 老版本（≤1.18）：`downloads.classifiers` + `natives` 映射
/// - 新版本（1.19+）：natives 是独立库条目，路径里带 `-natives-<os>`
pub fn extract_all(
    paths: &Paths,
    id: &str,
    version: &VersionJson,
    progress: &Progress,
    count_hint: usize,
) -> Result<(), String> {
    let natives_dir = paths.natives_dir(id);
    std::fs::create_dir_all(&natives_dir).map_err(|e| format!("创建 natives 目录失败: {e}"))?;

    let jars: Vec<PathBuf> = version
        .active_libraries()
        .filter_map(|lib| {
            let downloads = lib.downloads.as_ref()?;
            if let Some(key) = version.native_key(lib) {
                if let Some(native) = downloads.classifiers.get(&key) {
                    return native.path.as_ref().map(|p| paths.libraries().join(p));
                }
            }
            let path = downloads.artifact.as_ref()?.path.as_ref()?;
            if platform::is_native_artifact(path) && platform::is_usable(path) {
                Some(paths.libraries().join(path))
            } else {
                None
            }
        })
        .collect();

    let total = count_hint.max(jars.len()).max(1);
    for (i, jar) in jars.iter().enumerate() {
        if jar.is_file() {
            extract_jar(jar, &natives_dir)?;
        }
        progress.call("解压 natives", i + 1, total);
    }
    Ok(())
}

fn extract_jar(jar: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(jar).map_err(|e| format!("打开 {} 失败: {e}", jar.display()))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("解析 {} 失败: {e}", jar.display()))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("读取 {} 条目失败: {e}", jar.display()))?;
        if entry.is_dir() {
            continue;
        }
        let Some(path) = entry.enclosed_name() else {
            continue;
        };
        if EXCLUDES.iter().any(|p| path.starts_with(p)) {
            continue;
        }
        let out = dest.join(&path);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录 {} 失败: {e}", parent.display()))?;
        }
        let mut buffer = Vec::new();
        entry
            .read_to_end(&mut buffer)
            .map_err(|e| format!("读取条目失败: {e}"))?;
        std::fs::write(&out, buffer).map_err(|e| format!("写入 {} 失败: {e}", out.display()))?;
    }
    Ok(())
}
