//! 日志与崩溃报告：列表、读取尾部、崩溃摘要。

use super::CrashEntry;
use crate::paths::Paths;
use std::path::Path;

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

/// 统计版本目录：总大小、文件数、模组数与存档数。

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

pub(crate) fn format_time(time: std::time::SystemTime) -> String {
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
pub(crate) fn civil_from_days(z: i64) -> (i64, u32, u32) {
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
