//! 版本体积统计：目录大小与文件数。

use super::VersionStats;
use crate::paths::Paths;
use super::game_dir;
use super::mods::list_mods;
use super::saves::list_saves;
use std::path::Path;

pub fn version_stats(paths: &Paths, id: &str, isolate: bool) -> VersionStats {
    let root = paths.version_dir(id);
    let mut size = 0u64;
    let mut files = 0u64;
    collect_size(&root, &mut size, &mut files, 0);

    let game = game_dir(paths, id, isolate);
    VersionStats {
        path: root.display().to_string(),
        size,
        files,
        mods: list_mods(&game.join("mods")).iter().filter(|m| m.jar).count(),
        saves: list_saves(&game.join("saves")).len(),
    }
}

fn collect_size(dir: &Path, size: &mut u64, files: &mut u64, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            collect_size(&entry.path(), size, files, depth + 1);
        } else {
            *size += meta.len();
            *files += 1;
        }
    }
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
