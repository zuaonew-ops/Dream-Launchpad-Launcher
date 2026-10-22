//! 游戏内设置：读写 `options.txt`。
//!
//! 格式是每行 `key:value`，键顺序无所谓，游戏自己会重写整个文件。
//! 这里只编辑几个启动器常见关心的项，其余原样保留 —— 不能整个覆盖，
//! 否则会把游戏写进去的音量、按键绑定等全部抹掉。

use serde::{Deserialize, Serialize};
use std::path::Path;

/// 启动器里能改的那几项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameOptions {
    pub render_distance: u32,
    pub simulation_distance: u32,
    pub max_fps: u32,
    pub fov: f64,
    pub gui_scale: u32,
    pub gamma: f64,
    pub graphics: String,
    pub particles: String,
    pub language: String,
    pub fullscreen: bool,
    /// 原始文件里的行数（界面用来判断「有没有生成过」）
    pub exists: bool,
}

impl Default for GameOptions {
    fn default() -> Self {
        Self {
            render_distance: 12,
            simulation_distance: 12,
            max_fps: 120,
            fov: 70.0,
            gui_scale: 0,
            gamma: 0.5,
            graphics: "fancy".into(),
            particles: "all".into(),
            language: "zh_cn".into(),
            fullscreen: false,
            exists: false,
        }
    }
}

fn path(dir: &Path) -> std::path::PathBuf {
    dir.join("options.txt")
}

/// 读：缺的键用默认值。
pub fn read(dir: &Path) -> GameOptions {
    let mut options = GameOptions::default();
    let Ok(text) = std::fs::read_to_string(path(dir)) else {
        return options;
    };
    options.exists = true;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "renderDistance" => options.render_distance = value.parse().unwrap_or(options.render_distance),
            "simulationDistance" => {
                options.simulation_distance = value.parse().unwrap_or(options.simulation_distance)
            }
            "maxFps" => options.max_fps = value.parse().unwrap_or(options.max_fps),
            "fov" => options.fov = value.parse().unwrap_or(options.fov),
            "guiScale" => options.gui_scale = value.parse().unwrap_or(options.gui_scale),
            "gamma" => options.gamma = value.parse().unwrap_or(options.gamma),
            "graphics" => options.graphics = value.to_string(),
            "particles" => options.particles = value.to_string(),
            "lang" => options.language = value.to_string(),
            "fullscreen" => options.fullscreen = value == "true",
            _ => {}
        }
    }
    options
}

/// 写：只改认识的键，其余行原样保留（含顺序）。
pub fn write(dir: &Path, options: &GameOptions) -> Result<(), String> {
    let file = path(dir);
    let existing = std::fs::read_to_string(&file).unwrap_or_default();
    let mut lines: Vec<String> = existing.lines().map(|l| l.to_string()).collect();

    let updates = [
        ("renderDistance", options.render_distance.to_string()),
        ("simulationDistance", options.simulation_distance.to_string()),
        ("maxFps", options.max_fps.to_string()),
        ("fov", format!("{}", options.fov)),
        ("guiScale", options.gui_scale.to_string()),
        ("gamma", format!("{}", options.gamma)),
        ("graphics", options.graphics.clone()),
        ("particles", options.particles.clone()),
        ("lang", options.language.clone()),
        ("fullscreen", options.fullscreen.to_string()),
    ];

    for (key, value) in updates {
        let new_line = format!("{key}:{value}");
        match lines
            .iter()
            .position(|line| line.split(':').next() == Some(key))
        {
            Some(index) => lines[index] = new_line,
            None => lines.push(new_line),
        }
    }

    std::fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {e}"))?;
    let mut text = lines.join("\n");
    text.push('\n');
    std::fs::write(&file, text).map_err(|e| format!("写入 {} 失败: {e}", file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_unknown_keys() {
        let dir = crate::testdir::temp_dir("dlp-options");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("options.txt"),
            "version:3955\nrenderDistance:8\nkey_key.jump:key.keyboard.space\n",
        )
        .unwrap();

        let mut options = read(&dir);
        assert!(options.exists);
        assert_eq!(options.render_distance, 8);
        options.render_distance = 16;
        options.language = "zh_cn".into();
        write(&dir, &options).unwrap();

        let text = std::fs::read_to_string(dir.join("options.txt")).unwrap();
        assert!(text.contains("renderDistance:16"));
        // 不认识的行必须还在
        assert!(text.contains("key_key.jump:key.keyboard.space"));
        assert!(text.contains("version:3955"));
        assert!(text.contains("lang:zh_cn"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn defaults_when_missing() {
        let dir = crate::testdir::temp_dir("dlp-options2");
        let options = read(&dir);
        assert!(!options.exists);
        assert_eq!(options.render_distance, 12);
    }
}

/// 读写 `options.txt` 里的 `resourcePacks` / `incompatibleResourcePacks`。
///
/// 值是 JSON 数组字符串，元素形如 `"vanilla"`、`"file/我的包.zip"`。
/// 界面只让用户勾启用/停用，这里负责格式：进 `resourcePacks` 的就是启用的。
pub fn read_resource_packs(dir: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path(dir)) else {
        return Vec::new();
    };
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("resourcePacks:") else {
            continue;
        };
        let parsed: Vec<String> = serde_json::from_str(rest).unwrap_or_default();
        return parsed.into_iter().filter(|item| item != "vanilla").collect();
    }
    Vec::new()
}

/// 设置启用的资源包（整体覆盖）。`vanilla` 永远排在最前面（内置资源）。
pub fn write_resource_packs(dir: &Path, packs: &[String]) -> Result<(), String> {
    let file = path(dir);
    let existing = std::fs::read_to_string(&file).unwrap_or_default();
    let mut lines: Vec<String> = existing.lines().map(|l| l.to_string()).collect();

    let mut all = vec!["vanilla".to_string()];
    all.extend(packs.iter().cloned());
    let value = serde_json::to_string(&all).map_err(|e| format!("序列化失败: {e}"))?;
    let new_line = format!("resourcePacks:{value}");

    match lines
        .iter()
        .position(|line| line.starts_with("resourcePacks:"))
    {
        Some(index) => lines[index] = new_line,
        None => lines.push(new_line),
    }

    std::fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {e}"))?;
    let mut text = lines.join("\n");
    text.push('\n');
    std::fs::write(&file, text).map_err(|e| format!("写入 {} 失败: {e}", file.display()))
}

#[cfg(test)]
mod resource_tests {
    use super::*;

    #[test]
    fn reads_and_writes_resource_packs() {
        let dir = crate::testdir::temp_dir("dlp-respack");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("options.txt"),
            "version:3955\nresourcePacks:[\"vanilla\",\"file/我的包.zip\"]\n",
        )
        .unwrap();

        let packs = read_resource_packs(&dir);
        assert_eq!(packs, vec!["file/我的包.zip"]);

        write_resource_packs(&dir, &["file/另一个.zip".into()]).unwrap();
        let text = std::fs::read_to_string(dir.join("options.txt")).unwrap();
        // vanilla 要在，旧包要没，别的行要保住
        assert!(text.contains("resourcePacks:[\"vanilla\",\"file/另一个.zip\"]"));
        assert!(text.contains("version:3955"));
        assert_eq!(read_resource_packs(&dir), vec!["file/另一个.zip"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn works_without_options_file() {
        let dir = crate::testdir::temp_dir("dlp-respack2");
        assert!(read_resource_packs(&dir).is_empty());
        write_resource_packs(&dir, &[]).unwrap();
        assert!(read_resource_packs(&dir).is_empty());
    }
}
