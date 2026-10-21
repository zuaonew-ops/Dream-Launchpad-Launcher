//! 资源包 / 光影：列目录里的 zip 与文件夹，支持删除。

use super::ModEntry;
use std::path::Path;

pub fn list_packs(dir: &Path, kind: &str) -> Vec<ModEntry> {
    let base = dir.join(kind);
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&base) else {
        return out;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let is_file = path.is_file();
        if is_file && !(name.ends_with(".zip") || name.ends_with(".jar")) {
            continue;
        }
        let size = if is_file {
            entry.metadata().map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };
        out.push(ModEntry {
            name,
            size,
            enabled: true,
            jar: is_file,
        });
    }
    out.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    out
}

/// 删除资源包 / 光影（zip 或文件夹）
pub fn delete_pack(dir: &Path, kind: &str, name: &str) -> Result<(), String> {
    let target = dir.join(kind).join(name);
    if !target.exists() {
        return Err(format!("找不到 {name}"));
    }
    if target.is_dir() {
        std::fs::remove_dir_all(&target).map_err(|e| format!("删除目录失败: {e}"))
    } else {
        std::fs::remove_file(&target).map_err(|e| format!("删除文件失败: {e}"))
    }
}
