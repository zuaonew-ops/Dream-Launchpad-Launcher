//! 版本内容：mods 列表 / 存档列表 / 日志读取。
//!
//! 参考 PCL 的「模组管理 / 存档管理 / 日志」三件套，但只保留最常用的操作。

use crate::paths::Paths;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
pub struct ModEntry {
    pub name: String,
    pub size: u64,
    pub enabled: bool,
    /// 是否是可识别的模组文件（.jar / .jar.disabled）
    pub jar: bool,
}

/// 崩溃报告条目
#[derive(Serialize, Clone, Debug)]
pub struct CrashEntry {
    pub name: String,
    pub modified: String,
    pub size: u64,
    /// 报告里 Description / 首个异常行
    pub summary: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct SaveEntry {
    pub name: String,
    pub modified: String,
    /// 存在 level.dat 才算真正可用的存档
    pub valid: bool,
}

/// 该版本的游戏目录（版本隔离时在 versions/<id>/ 下）​
pub fn game_dir(paths: &Paths, id: &str, isolate: bool) -> PathBuf {
    if isolate {
        paths.version_dir(id)
    } else {
        paths.root.clone()
    }
}

/// 列出 mods：`.disabled` 结尾视为已禁用（PCL 也是这个约定）。
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

/// 启用 / 禁用模组：改后缀，不删文件。
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
pub fn list_saves(dir: &Path) -> Vec<SaveEntry> {
    let saves = dir.join("saves");
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&saves) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let valid = path.join("level.dat").is_file();
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .map(format_time)
            .unwrap_or_default();
        out.push(SaveEntry {
            name,
            modified,
            valid,
        });
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

/// 一次把所有模组启用 / 禁用，返回改动的文件数。
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

/// 崩溃报告（`crash-reports/crash-*.txt`）的摘要。
pub fn list_crashes(paths: &Paths) -> Vec<CrashEntry> {
    let dir = paths.root.join("crash-reports");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, String, u64, String)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .map(format_time)
            .unwrap_or_default();
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        out.push((name.clone(), modified, size, crash_summary(&path)));
    }
    // 文件名里带时间戳，按名字倒序就是新到旧
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.into_iter()
        .map(|(name, modified, size, summary)| CrashEntry {
            name,
            modified,
            size,
            summary,
        })
        .collect()
}

/// 读一份崩溃报告（整份，界面里自己滚）。
pub fn read_crash(paths: &Paths, name: &str) -> Result<String, String> {
    let path = paths.root.join("crash-reports").join(name);
    std::fs::read_to_string(&path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))
}

/// 从崩溃报告里挑一行最有信息量的（Description / 异常类型）。
fn crash_summary(path: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(path) else {
        return String::new();
    };
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Description:") {
            return rest.trim().to_string();
        }
    }
    // 没有 Description 就找第一个异常行
    for line in text.lines() {
        let line = line.trim();
        if line.contains("Exception") || line.contains("Error") {
            return line.chars().take(160).collect();
        }
    }
    String::new()
}

/// 读日志尾部（默认 400 行），给界面里的日志面板用。
pub fn read_log(paths: &Paths, name: &str, max_lines: usize) -> Result<String, String> {
    let path = paths.root.join("logs").join(name);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].join("\n"))
}

/// 可用日志文件列表（按修改时间倒序）
pub fn list_logs(paths: &Paths) -> Vec<String> {
    let dir = paths.root.join("logs");
    let mut out: Vec<(String, std::time::SystemTime)> = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        if !entry.path().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".log") && !name.ends_with(".txt") {
            continue;
        }
        let time = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        out.push((name, time));
    }
    out.sort_by(|a, b| b.1.cmp(&a.1));
    out.into_iter().map(|(name, _)| name).collect()
}

fn format_time(time: std::time::SystemTime) -> String {
    let secs = time
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 只给界面看，避免引入时间库：按 UTC 粗略格式化
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60
    )
}

/// Howard Hinnant 的 days → civil date 算法
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mods_toggle_by_suffix() {
        let dir = std::env::temp_dir().join(format!("dlp-mods-{}", std::process::id()));
        let mods = dir.join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("a.jar"), b"x").unwrap();

        set_mod_enabled(&dir, "a.jar", false).unwrap();
        assert!(mods.join("a.jar.disabled").is_file());
        let listed = list_mods(&dir);
        assert_eq!(listed.len(), 1);
        assert!(!listed[0].enabled);

        set_mod_enabled(&dir, "a.jar.disabled", true).unwrap();
        assert!(mods.join("a.jar").is_file());
        assert!(list_mods(&dir)[0].enabled);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saves_only_dirs_with_level_dat_are_valid() {
        let dir = std::env::temp_dir().join(format!("dlp-saves-{}", std::process::id()));
        let saves = dir.join("saves").join("world");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("level.dat"), b"x").unwrap();
        std::fs::create_dir_all(dir.join("saves").join("broken")).unwrap();

        let listed = list_saves(&dir);
        assert_eq!(listed.len(), 2);
        let world = listed.iter().find(|s| s.name == "world").unwrap();
        assert!(world.valid);
        assert!(!listed.iter().find(|s| s.name == "broken").unwrap().valid);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn log_tail_keeps_last_lines() {
        let dir = std::env::temp_dir().join(format!("dlp-logs-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("logs")).unwrap();
        std::fs::write(dir.join("logs").join("latest.log"), "1\n2\n3\n4\n").unwrap();
        let text = read_log(&Paths::new(&dir), "latest.log", 2).unwrap();
        assert_eq!(text, "3\n4");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
