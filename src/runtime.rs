//! Java 运行时下载（Adoptium Temurin）。
//!
//! 装到 `<root>/runtime/<发行名>/`，随后由 [`crate::java::detect`] 扫到。

use crate::net::{download, Job, Progress};
use crate::paths::Paths;
use reqwest::Client;
use serde::Serialize;
use std::io::Read;
use std::path::PathBuf;

const API: &str = "https://api.adoptium.net/v3";

#[derive(Serialize, Clone, Debug)]
pub struct Package {
    pub name: String,
    pub url: String,
    pub size: u64,
}

pub async fn latest(client: &Client, major: u32) -> Result<Package, String> {
    let os = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "mac",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        "x86" => "x86",
        other => other,
    };

    let url = format!("{API}/assets/latest/{major}/hotspot");
    let response = client
        .get(&url)
        .query(&[
            ("os", os.to_string()),
            ("architecture", arch.to_string()),
            ("image_type", "jre".to_string()),
            ("vendor", "eclipse".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("请求 Adoptium 失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Adoptium 返回 HTTP {}（可能没有 Java {major} 的包）", response.status()));
    }

    let body: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|e| format!("解析 Adoptium 响应失败: {e}"))?;
    let first = body.first().ok_or("Adoptium 没有返回可用包")?;

    let name = first
        .get("release_name")
        .and_then(|v| v.as_str())
        .unwrap_or("jre")
        .to_string();
    let package = first
        .get("binary")
        .and_then(|b| b.get("package"))
        .ok_or("响应里没有 package")?;
    let url = package
        .get("link")
        .and_then(|v| v.as_str())
        .ok_or("package 缺少 link")?
        .to_string();
    let size = package.get("size").and_then(|v| v.as_u64()).unwrap_or(0);

    Ok(Package { name, url, size })
}

/// 下载并解压到 `<root>/runtime/`，返回 java 可执行文件路径。
pub async fn install(
    client: &Client,
    paths: &Paths,
    major: u32,
    progress: Option<Progress>,
) -> Result<PathBuf, String> {
    let package = latest(client, major).await?;
    let dir = paths.runtime_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建 {} 失败: {e}", dir.display()))?;

    let archive = dir.join(format!("{}.zip", package.name));
    if !archive.is_file() {
        let job = Job::new(package.url.clone(), &archive, None);
        download::run(
            client,
            vec![job],
            1,
            "下载 Java",
            &progress.clone().unwrap_or_else(Progress::none),
        )
        .await?;
    }

    let target = dir.join(&package.name);
    if !target.is_dir() {
        extract_zip(&archive, &dir)?;
    }

    let java = find_java(&target).or_else(|| find_java(&dir)).ok_or_else(|| {
        format!(
            "解压后没找到 java 可执行文件（{}）",
            target.display()
        )
    })?;
    Ok(java)
}

fn extract_zip(archive: &std::path::Path, dir: &std::path::Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|e| format!("打开 {} 失败: {e}", archive.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("读取压缩包失败: {e}"))?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|e| format!("读取条目失败: {e}"))?;
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let out = dir.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).ok();
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut buffer = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buffer)
            .map_err(|e| format!("解压失败: {e}"))?;
        std::fs::write(&out, buffer).map_err(|e| format!("写入 {} 失败: {e}", out.display()))?;
    }
    Ok(())
}

fn find_java(root: &std::path::Path) -> Option<PathBuf> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let direct = root.join("bin").join(exe);
    if direct.is_file() {
        return Some(direct);
    }
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let candidate = entry.path().join("bin").join(exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
