//! 存档备份：把一个世界压成 zip 放到 `dlp-backups/`，能还原回去。
//!
//! 为什么不用「复制文件夹」：存档动辄几百 MB，里面大量小文件，
//! 打包成一个 zip 更快、也便于整个删掉。备份文件按 `世界名-时间.zip` 命名。

use super::logs::format_time;
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// 备份目录名（放在游戏目录下，和 saves 平级）。
pub const BACKUP_DIR: &str = "dlp-backups";

#[derive(Debug, Clone, Serialize)]
pub struct BackupEntry {
    /// 备份文件名（含 .zip）
    pub name: String,
    /// 备份的是哪个世界
    pub world: String,
    pub time: String,
    pub bytes: u64,
}

/// 备份一个世界，返回备份文件名。
pub fn backup(dir: &Path, world: &str) -> Result<String, String> {
    let src = dir.join("saves").join(world);
    if !src.is_dir() {
        return Err(format!("找不到存档 {world}"));
    }
    let out_dir = dir.join(BACKUP_DIR);
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("创建备份目录失败: {e}"))?;

    let stamp = chrono_like_now();
    let file_name = format!("{world}-{stamp}.zip");
    let target = out_dir.join(&file_name);

    let file = std::fs::File::create(&target)
        .map_err(|e| format!("创建 {} 失败: {e}", target.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    add_dir(&mut zip, &src, &src, options)?;
    zip.finish().map_err(|e| format!("收尾失败: {e}"))?;

    Ok(file_name)
}

/// 递归把目录塞进 zip（保留相对路径）。
fn add_dir(
    zip: &mut zip::ZipWriter<std::fs::File>,
    root: &Path,
    current: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    let entries =
        std::fs::read_dir(current).map_err(|e| format!("读 {} 失败: {e}", current.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .map_err(|e| format!("路径不在存档目录里: {e}"))?;
        // zip 里统一用正斜杠
        let name = rel.to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            zip.add_directory(format!("{name}/"), options)
                .map_err(|e| format!("写入目录 {name} 失败: {e}"))?;
            add_dir(zip, root, &path, options)?;
        } else {
            zip.start_file(&name, options)
                .map_err(|e| format!("写入 {name} 失败: {e}"))?;
            let mut f = std::fs::File::open(&path)
                .map_err(|e| format!("打开 {} 失败: {e}", path.display()))?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)
                .map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
            zip.write_all(&buf)
                .map_err(|e| format!("压缩 {name} 失败: {e}"))?;
        }
    }
    Ok(())
}

/// 列出备份（按时间倒序）。
pub fn list_backups(dir: &Path) -> Vec<BackupEntry> {
    let out_dir = dir.join(BACKUP_DIR);
    let Ok(entries) = std::fs::read_dir(&out_dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("zip") {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let time = entry
            .metadata()
            .and_then(|m| m.modified())
            .map(format_time)
            .unwrap_or_default();
        // 文件名是「世界名-年月日-时分秒.zip」，世界名本身可能带 '-'，从右边切两段
        let stem = name.trim_end_matches(".zip");
        let world = split_world(stem);
        out.push(BackupEntry {
            name,
            world,
            time,
            bytes,
        });
    }
    out.sort_by(|a, b| b.time.cmp(&a.time).then_with(|| b.name.cmp(&a.name)));
    out
}

/// `World-20261022-073000` → `World`
fn split_world(stem: &str) -> String {
    // 备份名是 `<世界名>-<年月日>-<时分秒>`，从右切两段，剩下整段都是世界名
    let mut parts = stem.rsplitn(3, '-');
    let _time = parts.next();
    let _date = parts.next();
    match parts.next() {
        Some(world) => world.to_string(),
        None => stem.to_string(),
    }
}

/// 还原一个备份到 `saves/<世界名>`；同名世界会被覆盖。
pub fn restore(dir: &Path, backup_name: &str) -> Result<String, String> {
    let file_path = dir.join(BACKUP_DIR).join(backup_name);
    let file = std::fs::File::open(&file_path)
        .map_err(|e| format!("打开 {} 失败: {e}", file_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("解析 {} 失败: {e}", file_path.display()))?;

    let world = split_world(backup_name.trim_end_matches(".zip"));
    let target = dir.join("saves").join(&world);
    if target.exists() {
        std::fs::remove_dir_all(&target).map_err(|e| format!("清理旧存档失败: {e}"))?;
    }
    std::fs::create_dir_all(&target).map_err(|e| format!("创建存档目录失败: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.name().replace('\\', "/");
        if name.is_empty() {
            continue;
        }
        let out = target.join(&name);
        if entry.is_dir() {
            let _ = std::fs::create_dir_all(&out);
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
        }
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|e| format!("读取 {name} 失败: {e}"))?;
        std::fs::write(&out, buf).map_err(|e| format!("写入 {} 失败: {e}", out.display()))?;
    }
    Ok(world)
}

/// 删一个备份。
pub fn delete_backup(dir: &Path, name: &str) -> Result<(), String> {
    let path: PathBuf = dir.join(BACKUP_DIR).join(name);
    std::fs::remove_file(&path).map_err(|e| format!("删除 {} 失败: {e}", path.display()))
}

/// `20261022-073000`：备份名里用它，排序即时间序。
fn chrono_like_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (year, month, day) = super::logs::civil_from_days(days as i64);
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_world_from_backup_name() {
        assert_eq!(split_world("My World-20261022-073000"), "My World");
        assert_eq!(split_world("a-b-20261022-073000"), "a-b");
    }

    #[test]
    fn backup_then_restore_roundtrip() {
        let dir = std::env::temp_dir().join(format!("dlp-backup-{}", std::process::id()));
        let world = dir.join("saves").join("世界");
        std::fs::create_dir_all(world.join("region")).unwrap();
        std::fs::write(world.join("level.dat"), b"data").unwrap();
        std::fs::write(world.join("region").join("r.0.0.mca"), b"mca").unwrap();

        let name = backup(&dir, "世界").unwrap();
        assert!(dir.join(BACKUP_DIR).join(&name).is_file());
        let listed = list_backups(&dir);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].world, "世界");

        // 改坏原存档，再还原
        std::fs::write(world.join("level.dat"), b"broken").unwrap();
        let restored = restore(&dir, &name).unwrap();
        assert_eq!(restored, "世界");
        assert_eq!(
            std::fs::read(world.join("level.dat")).unwrap(),
            b"data".to_vec()
        );
        assert!(world.join("region").join("r.0.0.mca").is_file());

        delete_backup(&dir, &name).unwrap();
        assert!(list_backups(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
