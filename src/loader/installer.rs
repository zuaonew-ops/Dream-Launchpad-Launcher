//! Forge / NeoForge：下载官方 installer 并用 Java 静默安装。

use super::run::{copy_dir, get_text, run_java, run_java_raw, tail, version_ids};
use super::{Candidate, LoaderKind, BMCLAPI};
use crate::net::{download, Job, Progress};
use crate::paths::Paths;
use reqwest::Client;
use std::path::{Path, PathBuf};

pub async fn installer_candidates(
    client: &Client,
    mc: &str,
    kind: LoaderKind,
) -> Result<Vec<Candidate>, String> {
    let url = match kind {
        LoaderKind::Forge => format!("{BMCLAPI}/forge/minecraft/{mc}"),
        LoaderKind::NeoForge => format!("{BMCLAPI}/neoforge/list/{mc}"),
        _ => return Err("这个加载器没有 installer".into()),
    };
    let body = get_text(client, &url).await?;
    let list: Vec<serde_json::Value> =
        serde_json::from_str(&body).map_err(|e| format!("解析 {url} 失败: {e}"))?;

    let mut out: Vec<Candidate> = list
        .iter()
        .filter_map(|item| {
            let version = item.get("version")?.as_str()?.to_string();
            Some(Candidate {
                label: version.clone(),
                version,
                stable: true,
            })
        })
        .collect();
    out.reverse();
    Ok(out)
}

fn installer_url(kind: LoaderKind, mc: &str, version: &str) -> String {
    match kind {
        LoaderKind::Forge => format!(
            "{BMCLAPI}/maven/net/minecraftforge/forge/{mc}-{version}/forge-{mc}-{version}-installer.jar"
        ),
        _ => format!(
            "{BMCLAPI}/maven/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar"
        ),
    }
}

/// 从接口里翻出某个版本的 installerPath（BMCLAPI 会给 /maven/... 的相对路径）
pub async fn resolve_installer_path(
    client: &Client,
    mc: &str,
    kind: LoaderKind,
    version: &str,
) -> Option<String> {
    let url = match kind {
        LoaderKind::Forge => format!("{BMCLAPI}/forge/minecraft/{mc}"),
        LoaderKind::NeoForge => format!("{BMCLAPI}/neoforge/list/{mc}"),
        _ => return None,
    };
    let body = get_text(client, &url).await.ok()?;
    let list: Vec<serde_json::Value> = serde_json::from_str(&body).ok()?;
    list.iter().find_map(|item| {
        let same = item.get("version").and_then(|v| v.as_str()) == Some(version)
            || item.get("rawVersion").and_then(|v| v.as_str()) == Some(version);
        if !same {
            return None;
        }
        item.get("installerPath")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    })
}

/// 下安装器 → 跑 `--installClient` → 返回生成的版本 id。
pub async fn install_with_installer(
    client: &Client,
    paths: &Paths,
    mc: &str,
    kind: LoaderKind,
    version: &str,
    installer_path: Option<String>,
    java: &Path,
    progress: Option<Progress>,
) -> Result<String, String> {
    let dir = paths.root.join("dlp").join("installers");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {e}"))?;
    let jar = dir.join(format!("{}-{version}-installer.jar", kind.as_str()));

    if !jar.is_file() {
        let url = match installer_path {
            Some(path) if path.starts_with('/') => format!("{BMCLAPI}{path}"),
            Some(full) => full,
            None => installer_url(kind, mc, version),
        };
        let job = Job::new(url, &jar, None);
        download::run(
            client,
            vec![job],
            1,
            "下载安装器",
            &progress.clone().unwrap_or_else(Progress::none),
        )
        .await?;
    }

    // Forge / NeoForge 的安装器要求目标目录里有 launcher_profiles.json
    let profiles = paths.root.join("launcher_profiles.json");
    if !profiles.is_file() {
        std::fs::write(&profiles, r#"{"profiles":{},"settings":{},"version":3}"#)
            .map_err(|e| format!("写入 launcher_profiles.json 失败: {e}"))?;
    }

    let before = version_ids(paths);
    let (ok, output) = run_java_raw(
        java,
        &[
            "-Djava.awt.headless=true".into(),
            "-jar".into(),
            jar.to_string_lossy().to_string(),
            "--installClient".into(),
            paths.root.to_string_lossy().to_string(),
        ],
        &paths.root,
        &[],
    )?;
    let after = version_ids(paths);

    let created: Vec<String> = after.into_iter().filter(|id| !before.contains(id)).collect();
    if let Some(id) = created
        .iter()
        .find(|id| id.contains(&kind.as_str()) || id.contains("forge") || id.contains("neoforge"))
    {
        return Ok(id.clone());
    }
    if let Some(id) = created.first() {
        return Ok(id.clone());
    }

    Err(format!(
        "安装器没有生成新版本（退出码{}）。输出：\n{}",
        if ok { " 0" } else { " 非 0" },
        tail(&output, 1200)
    ))
}

// ---------------- OptiFine ----------------


/// 按安装器日志猜它新装了哪些版本目录（Forge 的日志里会带版本号）。
pub(crate) fn parse_installed(ids: &[String]) -> Vec<String> {
    ids.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_urls_match_maven_layout() {
        assert!(installer_url(LoaderKind::Forge, "1.20.1", "47.2.0")
            .contains("forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-installer.jar"));
        assert!(installer_url(LoaderKind::NeoForge, "1.21.1", "21.1.72")
            .contains("neoforge/21.1.72/neoforge-21.1.72-installer.jar"));
    }
}
