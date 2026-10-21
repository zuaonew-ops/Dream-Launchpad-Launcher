//! 打开链接 / 目录，以及各命令共用的小工具。

use dlp_core::{market, paths::Paths};
use std::path::{Path, PathBuf};

/// 用系统默认浏览器打开链接。
///
/// Windows 上用 ShellExecuteW —— 不走 `cmd /C start`，所以不会闪黑框。
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err(format!("只允许打开 https 链接：{url}"));
    }
    open_external(&url)
}

/// 在 MC 百科里找到某个模组的项目页。
///
/// mcmod 没有公开 API，只能走站内搜索页，再从 HTML 里取第一个
/// `https://www.mcmod.cn/class/NNNN.html` 链接。找不到就返回 None，
/// 前端会退回到搜索页。
#[tauri::command]
pub async fn mcmod_url(title: String) -> Result<Option<String>, String> {
    if title.trim().is_empty() {
        return Ok(None);
    }

    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .build()
        .map_err(|e| e.to_string())?;

    let body = client
        .get("https://search.mcmod.cn/s")
        .query(&[("key", title.as_str())])
        .send()
        .await
        .map_err(|e| format!("访问 MC 百科失败: {e}"))?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    Ok(first_mcmod_class(&body))
}

/// 从搜索页 HTML 里取第一个项目链接
fn first_mcmod_class(html: &str) -> Option<String> {
    const PREFIX: &str = "https://www.mcmod.cn/class/";
    let start = html.find(PREFIX)? + PREFIX.len();
    let rest = &html[start..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() || !rest[digits.len()..].starts_with(".html") {
        return None;
    }
    Some(format!("{PREFIX}{digits}.html"))
}

#[cfg(target_os = "windows")]
fn open_external(url: &str) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide = |text: &str| -> Vec<u16> {
        OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };
    let operation = wide("open");
    let target = wide(url);
    // SAFETY: 两个字符串都以 NUL 结尾，且活着直到调用结束
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        return Err(format!("打开浏览器失败（ShellExecute 返回 {}）", result as isize));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_external(url: &str) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("打开浏览器失败: {e}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_external(url: &str) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("打开浏览器失败: {e}"))
}

/// 用资源管理器打开一个目录（不存在就建）
pub fn open_path(dir: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("打开目录失败: {e}"))
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("打开目录失败: {e}"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("打开目录失败: {e}"))
    }
}

/// 游戏实际使用的目录：开了隔离就是这个版本自己的目录，否则是 .minecraft 根
pub fn game_dir(paths: &Paths, id: &str) -> PathBuf {
    let isolate = dlp_core::settings::Store::load(paths)
        .merged(id)
        .isolate
        .unwrap_or(true);
    if isolate {
        paths.version_dir(id)
    } else {
        paths.root.clone()
    }
}

/// 从版本 id 推断加载器：`fabric-loader-0.15.11-1.20.1` → fabric
pub fn loader_of(id: &str) -> Option<String> {
    let lower = id.to_ascii_lowercase();
    for key in ["fabric", "quilt", "neoforge", "forge", "optifine"] {
        if lower.contains(key) {
            return Some(key.to_string());
        }
    }
    None
}

/// 版本对应的 MC 版本号（加载器版本 → 基础版本）
pub fn base_mc_version(paths: &Paths, id: &str) -> String {
    if let Ok(version) = dlp_core::minecraft::load_version(paths, id) {
        if let Some(base) = version.inherits_from {
            return base;
        }
    }
    id.to_string()
}

/// 前端传字符串，后端映射成枚举；不认识就按 Modrinth
pub fn market_source(value: Option<String>) -> market::Source {
    match value.as_deref() {
        Some("curseforge") => market::Source::CurseForge,
        _ => market::Source::Modrinth,
    }
}

/// CurseForge 的 key 存在 app 设置里；没填就是空串（调用时会给出可读错误）
pub fn curseforge_key(paths: &Paths) -> String {
    dlp_core::settings::AppSettings::load(paths)
        .curseforge_api_key
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::first_mcmod_class;

    #[test]
    fn finds_first_class_link() {
        let html = r#"<a href="https://www.mcmod.cn/class/8714.html">x</a><a href="https://www.mcmod.cn/class/22671.html">y</a>"#;
        assert_eq!(
            first_mcmod_class(html).as_deref(),
            Some("https://www.mcmod.cn/class/8714.html")
        );
    }

    #[test]
    fn none_without_link() {
        assert_eq!(first_mcmod_class("<html></html>"), None);
    }
}
