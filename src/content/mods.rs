//! 模组：列出、启用/禁用（改 .disabled 后缀）、删除、批量开关。

use super::ModEntry;
use std::path::Path;

pub fn list_mods(dir: &Path) -> Vec<ModEntry> {
    let mods = dir.join("mods");
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&mods) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let enabled = !name.ends_with(".disabled");
        let jar = name.ends_with(".jar") || name.ends_with(".jar.disabled");
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        out.push(ModEntry {
            name,
            size,
            enabled,
            jar,
        });
    }
    out.sort_by(|a, b| b.enabled.cmp(&a.enabled).then_with(|| a.name.cmp(&b.name)));
    out
}

/// 列出资源包 / 光影：只看直接放在目录里的文件或文件夹。
///
/// 资源包可以是 zip 也可以是文件夹，两种都列。

pub fn set_mod_enabled(dir: &Path, name: &str, enabled: bool) -> Result<(), String> {
    let base = dir.join("mods");
    let from = base.join(name);
    if !from.is_file() {
        return Err(format!("找不到模组 {name}"));
    }
    let target_name = if enabled {
        name.trim_end_matches(".disabled").to_string()
    } else if name.ends_with(".disabled") {
        name.to_string()
    } else {
        format!("{name}.disabled")
    };
    if target_name == name {
        return Ok(());
    }
    let to = base.join(target_name);
    std::fs::rename(&from, &to).map_err(|e| format!("重命名失败: {e}"))
}

pub fn delete_mod(dir: &Path, name: &str) -> Result<(), String> {
    let path = dir.join("mods").join(name);
    std::fs::remove_file(&path).map_err(|e| format!("删除 {name} 失败: {e}"))
}

/// 存档列表：`saves/` 下的目录，有 level.dat 的算有效存档。

pub fn set_all_mods(dir: &Path, enabled: bool) -> Result<usize, String> {
    let mut changed = 0;
    for entry in list_mods(dir) {
        let currently = entry.enabled;
        if currently != enabled {
            set_mod_enabled(dir, &entry.name, enabled)?;
            changed += 1;
        }
    }
    Ok(changed)
}
