//! 游戏截图：列出 `screenshots/` 里的图，能删、能用系统看图打开。

use super::logs::format_time;
use super::skins::png_size;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct ShotEntry {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub time: String,
}

/// 列截图（新的在前）。
pub fn list_shots(dir: &Path) -> Vec<ShotEntry> {
    let shots = dir.join("screenshots");
    let Ok(entries) = std::fs::read_dir(&shots) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg") {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let time = entry
            .metadata()
            .and_then(|m| m.modified())
            .map(format_time)
            .unwrap_or_default();
        let (width, height) = png_size(&path).unwrap_or((0, 0));
        out.push(ShotEntry {
            name,
            width,
            height,
            bytes,
            time,
        });
    }
    // 文件名里带时间戳（2026-10-22_12.30.00.png），按名字倒序就是新到旧
    out.sort_by(|a, b| b.name.cmp(&a.name));
    out
}

/// 删一张截图。
pub fn delete_shot(dir: &Path, name: &str) -> Result<(), String> {
    let path = dir.join("screenshots").join(name);
    std::fs::remove_file(&path).map_err(|e| format!("删除 {} 失败: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_png_and_jpg_only() {
        let dir = crate::testdir::temp_dir("dlp-shots");
        let shots = dir.join("screenshots");
        std::fs::create_dir_all(&shots).unwrap();
        std::fs::write(shots.join("2026-10-22_12.30.00.png"), super::super::skins::make_png(64, 64))
            .unwrap();
        std::fs::write(shots.join("2026-10-22_12.31.00.jpg"), b"not really jpg").unwrap();
        std::fs::write(shots.join("notes.txt"), b"x").unwrap();

        let list = list_shots(&dir);
        assert_eq!(list.len(), 2);
        // 名字倒序：12.31 在前
        assert_eq!(list[0].name, "2026-10-22_12.31.00.jpg");
        assert_eq!(list[1].width, 64);

        delete_shot(&dir, "2026-10-22_12.31.00.jpg").unwrap();
        assert_eq!(list_shots(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
