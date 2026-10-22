//! 皮肤：管理一组皮肤文件，并接到 CustomSkinLoader 的本地皮肤目录。
//!
//! 离线账号没法用正版皮肤服务器，社区通行的做法是装 CustomSkinLoader 这个模组，
//! 它读 `<游戏目录>/CustomSkinLoader/LocalSkin/skins/<玩家名>.png`。
//! 这里就干两件事：存皮肤、按玩家名放到位。

use serde::Serialize;
use std::path::Path;

/// 皮肤库目录（和 saves 平级）。
pub const SKIN_DIR: &str = "dlp-skins";
/// CustomSkinLoader 读本地皮肤的目录。
const CSL_SKINS: &str = "CustomSkinLoader/LocalSkin/skins";

#[derive(Debug, Clone, Serialize)]
pub struct SkinEntry {
    pub name: String,
    /// 宽 × 高（64×64 是新版格式，64×32 是旧版）
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
}

/// 列皮肤库。
pub fn list_skins(dir: &Path) -> Vec<SkinEntry> {
    let skin_dir = dir.join(SKIN_DIR);
    let Ok(entries) = std::fs::read_dir(&skin_dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("png") {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let (width, height) = png_size(&path).unwrap_or((0, 0));
        out.push(SkinEntry {
            name,
            width,
            height,
            bytes,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// 导入一个皮肤文件（必须是 PNG）。
pub fn import_skin(dir: &Path, source: &Path, name: &str) -> Result<String, String> {
    let (width, height) = png_size(source)
        .ok_or_else(|| format!("{} 不是 PNG 图片", source.display()))?;
    if width != 64 || (height != 64 && height != 32) {
        return Err(format!(
            "皮肤尺寸应该是 64×64 或 64×32，这个是 {width}×{height}"
        ));
    }
    let file_name = if name.to_ascii_lowercase().ends_with(".png") {
        name.to_string()
    } else {
        format!("{name}.png")
    };
    let skin_dir = dir.join(SKIN_DIR);
    std::fs::create_dir_all(&skin_dir).map_err(|e| format!("创建皮肤目录失败: {e}"))?;
    let target = skin_dir.join(&file_name);
    std::fs::copy(source, &target).map_err(|e| format!("复制皮肤失败: {e}"))?;
    Ok(file_name)
}

/// 删一个皮肤。
pub fn delete_skin(dir: &Path, name: &str) -> Result<(), String> {
    let path = dir.join(SKIN_DIR).join(name);
    std::fs::remove_file(&path).map_err(|e| format!("删除 {} 失败: {e}", path.display()))
}

/// 这个版本装没装 CustomSkinLoader。
pub fn has_customskinloader(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir.join("mods")) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        name.starts_with("customskinloader") && name.contains(".jar")
    })
}

/// 把皮肤装成某个玩家的：写到 CustomSkinLoader 的本地皮肤目录。
pub fn apply_skin(dir: &Path, skin: &str, player: &str) -> Result<String, String> {
    if player.trim().is_empty() {
        return Err("要指定玩家名（游戏里的名字）".into());
    }
    let from = dir.join(SKIN_DIR).join(skin);
    if !from.is_file() {
        return Err(format!("皮肤库里没有 {skin}"));
    }
    let target_dir = dir.join(CSL_SKINS);
    std::fs::create_dir_all(&target_dir).map_err(|e| format!("创建目录失败: {e}"))?;
    let target = target_dir.join(format!("{}.png", player.trim()));
    std::fs::copy(&from, &target).map_err(|e| format!("写入皮肤失败: {e}"))?;
    Ok(target.to_string_lossy().to_string())
}

/// 读 PNG 的宽高（IHDR 里 8 字节开始的两个大端 u32）。
pub(crate) fn png_size(path: &Path) -> Option<(u32, u32)> {
    let data = std::fs::read(path).ok()?;
    // 8 字节签名 + 4 字节长度 + 4 字节 "IHDR"
    if data.len() < 24 || &data[1..4] != b"PNG" || &data[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((width, height))
}

/// 造一张 64×64 的纯色 PNG（测试用：不起图片库也能验证解析）。
#[cfg(test)]
pub(crate) fn make_png(width: u32, height: u32) -> Vec<u8> {
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    // IHDR
    out.extend_from_slice(&13u32.to_be_bytes());
    out.extend_from_slice(b"IHDR");
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&[8, 6, 0, 0, 0]);
    // CRC 随便填：只测尺寸解析，不测校验
    out.extend_from_slice(&[0, 0, 0, 0]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_png_size() {
        let dir = crate::testdir::temp_dir("dlp-skin");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.png");
        std::fs::write(&file, make_png(64, 64)).unwrap();
        assert_eq!(png_size(&file), Some((64, 64)));
    }

    #[test]
    fn imports_and_applies_skin() {
        let dir = crate::testdir::temp_dir("dlp-skin2");
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("src.png");
        std::fs::write(&src, make_png(64, 32)).unwrap();

        let name = import_skin(&dir, &src, "我").unwrap();
        assert_eq!(name, "我.png");
        let listed = list_skins(&dir);
        assert_eq!(listed.len(), 1);
        assert_eq!((listed[0].width, listed[0].height), (64, 32));

        let placed = apply_skin(&dir, &name, "Steve").unwrap();
        assert!(std::path::Path::new(&placed).is_file());
        assert!(placed.contains("LocalSkin"));

        delete_skin(&dir, &name).unwrap();
        assert!(list_skins(&dir).is_empty());
    }

    #[test]
    fn rejects_wrong_size() {
        let dir = crate::testdir::temp_dir("dlp-skin3");
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("bad.png");
        std::fs::write(&src, make_png(128, 128)).unwrap();
        assert!(import_skin(&dir, &src, "bad").is_err());
    }
}
