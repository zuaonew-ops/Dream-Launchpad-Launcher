//! 内容管理：模组、资源包、存档、日志、崩溃报告、体积统计。
//!
//! 按「管什么」拆成几个文件，每个文件只关心一类文件。

use crate::paths::Paths;
use serde::Serialize;
use std::path::PathBuf;

pub mod backup;
pub mod logs;
pub mod modmeta;
pub mod mods;
pub mod packs;
pub mod ping;
pub mod saves;
pub mod servers;
pub mod shots;
pub mod skins;
pub mod stats;

pub use backup::{backup, delete_backup, list_backups, restore, BackupEntry};
pub use logs::{list_crashes, list_logs, read_crash, read_log};
pub use modmeta::{duplicates, scan as scan_mods, DuplicateGroup, ModMeta};
pub use mods::{delete_mod, list_mods, set_all_mods, set_mod_enabled};
pub use packs::{delete_pack, list_packs};
pub use ping::{ping, ServerStatus};
pub use saves::list_saves;
pub use servers::{add_server, read_servers, remove_server, write_servers, ServerEntry};
pub use shots::{delete_shot, list_shots, ShotEntry};
pub use skins::{apply_skin, delete_skin, has_customskinloader, import_skin, list_skins, SkinEntry};
pub use stats::version_stats;

#[derive(Debug, Clone, Serialize)]
pub struct ModEntry {
    pub name: String,
    pub size: u64,
    pub enabled: bool,
    /// .jar 结尾（用来和文件夹区分）
    pub jar: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveEntry {
    pub name: String,
    pub valid: bool,
    pub modified: String,
}

/// 崩溃报告（`crash-reports/crash-*.txt`）的摘要。
#[derive(Debug, Clone, Serialize)]
pub struct CrashEntry {
    pub name: String,
    pub modified: String,
    pub size: u64,
    pub summary: String,
}

/// 一个版本的体积统计。
#[derive(Debug, Clone, Default, Serialize)]
pub struct VersionStats {
    pub path: String,
    pub size: u64,
    pub files: u64,
    pub mods: usize,
    pub saves: usize,
}

pub fn game_dir(paths: &Paths, id: &str, isolate: bool) -> PathBuf {
    if isolate {
        paths.version_dir(id)
    } else {
        paths.root.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mods_toggle_by_suffix() {
        let dir = crate::testdir::temp_dir("dlp-mods");
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
        let dir = crate::testdir::temp_dir("dlp-saves");
        let saves = dir.join("saves");
        std::fs::create_dir_all(saves.join("good")).unwrap();
        std::fs::write(saves.join("good").join("level.dat"), b"x").unwrap();
        std::fs::create_dir_all(saves.join("bad")).unwrap();

        let listed = list_saves(&dir);
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().any(|s| s.name == "good" && s.valid));
        assert!(listed.iter().any(|s| s.name == "bad" && !s.valid));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn log_tail_keeps_last_lines() {
        let dir = crate::testdir::temp_dir("dlp-logs");
        let logs = dir.join("logs");
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::write(logs.join("latest.log"), "a\nb\nc\n").unwrap();

        let paths = Paths::new(&dir);
        let tail = read_log(&paths, "latest.log", 2).unwrap();
        assert_eq!(tail, "b\nc");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
