//! 存档：每个世界一个文件夹，有 level.dat 才算有效。

use super::SaveEntry;
use super::logs::format_time;
use std::path::Path;

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
