//! 跑 Java 安装器的公共件：取文本、跑进程、拷目录。

use crate::paths::Paths;
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(crate) async fn get_text(client: &Client, url: &str) -> Result<String, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("请求 {url} 失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("{url} 返回 HTTP {}", response.status()));
    }
    response.text().await.map_err(|e| e.to_string())
}

pub(crate) fn version_ids(paths: &Paths) -> Vec<String> {
    std::fs::read_dir(paths.versions())
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// 跑 Java 并收集输出（Windows 上不弹黑框）。
pub(crate) fn run_java(java: &Path, args: &[String], cwd: &Path) -> Result<String, String> {
    let (ok, text) = run_java_raw(java, args, cwd, &[])?;
    if ok {
        Ok(text)
    } else {
        Err(format!("安装器失败：\n{}", tail(&text, 1500)))
    }
}

pub(crate) fn run_java_raw(
    java: &Path,
    args: &[String],
    cwd: &Path,
    envs: &[(&str, String)],
) -> Result<(bool, String), String> {
    let mut command = Command::new(java);
    for (key, value) in envs {
        command.env(key, value);
    }
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    let output = command
        .output()
        .map_err(|e| format!("启动 Java 失败（{}）: {e}", java.display()))?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    // 安装器可能在装完后弹对话框（headless 下抛异常），所以退出码不能当唯一判据
    Ok((output.status.success(), text))
}

pub(crate) fn tail(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_string();
    }
    chars[chars.len() - max..].iter().collect()
}

pub(crate) fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    if !from.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(to).map_err(|e| format!("创建 {} 失败: {e}", to.display()))?;
    for entry in std::fs::read_dir(from)
        .map_err(|e| format!("读取 {} 失败: {e}", from.display()))?
        .flatten()
    {
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest)
                .map_err(|e| format!("复制 {} 失败: {e}", path.display()))?;
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn _assert_send(_: PathBuf) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_keeps_last_chars() {
        assert_eq!(tail("abcdef", 3), "def");
    }
}
