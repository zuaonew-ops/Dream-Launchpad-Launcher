//! 模组加载器与 OptiFine 安装。
//!
//! 三条路线：
//! - **Fabric / Quilt**：官方 meta 直接给 profile JSON，落盘后复用原版安装逻辑，不需要跑安装器
//! - **Forge / NeoForge**：官方 installer 支持 `--installClient`，用 Java 静默跑
//! - **OptiFine**：安装器只认 `~/.minecraft`，所以给它一个临时 home，装完把产物搬回来
//!
//! 文件划分：本文件放类型与 Fabric/Quilt；`installer.rs` 放 Forge/NeoForge；
//! `optifine.rs` 放 OptiFine；`run.rs` 放跑 Java 安装器与拷目录的公共件。

use crate::install;
use crate::net::{download, Job, Progress};
use crate::paths::Paths;
use reqwest::Client;
use serde::Serialize;
use std::path::Path;

mod installer;
mod optifine;
pub(crate) mod run;

pub use installer::{install_with_installer, installer_candidates, resolve_installer_path};
pub use optifine::{install_optifine, optifine_candidates};
pub(crate) use run::get_text;

pub(crate) const BMCLAPI: &str = "https://bmclapi2.bangbang93.com";
pub(crate) const FABRIC_META: &str = "https://meta.fabricmc.net/v2";
pub(crate) const QUILT_META: &str = "https://meta.quiltmc.org/v3";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum LoaderKind {
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl LoaderKind {
    pub fn parse(text: &str) -> Option<Self> {
        match text.to_ascii_lowercase().as_str() {
            "fabric" => Some(Self::Fabric),
            "quilt" => Some(Self::Quilt),
            "forge" => Some(Self::Forge),
            "neoforge" | "neo" => Some(Self::NeoForge),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fabric => "fabric",
            Self::Quilt => "quilt",
            Self::Forge => "forge",
            Self::NeoForge => "neoforge",
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Candidate {
    /// 写进版本 id / 下载地址用的原始版本号
    pub version: String,
    /// 界面上显示的文案
    pub label: String,
    pub stable: bool,
}

// ---------------- Fabric / Quilt ----------------

pub async fn candidates(client: &Client, mc: &str, kind: LoaderKind) -> Result<Vec<Candidate>, String> {
    let base = match kind {
        LoaderKind::Fabric => FABRIC_META,
        LoaderKind::Quilt => QUILT_META,
        _ => return Err("这个加载器不走 meta 接口".into()),
    };
    let url = format!("{base}/versions/loader/{mc}");
    let body = get_text(client, &url).await?;
    let list: Vec<serde_json::Value> =
        serde_json::from_str(&body).map_err(|e| format!("解析 {url} 失败: {e}"))?;

    let mut out: Vec<Candidate> = list
        .iter()
        .filter_map(|item| {
            let version = item
                .get("loader")?
                .get("version")?
                .as_str()?
                .to_string();
            // Quilt 的 meta 没有 stable 字段，用版本号里的 beta/alpha/pre 兜底
            let lowered = version.to_ascii_lowercase();
            let looks_stable = !lowered.contains("beta")
                && !lowered.contains("alpha")
                && !lowered.contains("-pre")
                && !lowered.contains("rc");
            let stable = item
                .get("loader")
                .and_then(|l| l.get("stable"))
                .and_then(|s| s.as_bool())
                .unwrap_or(looks_stable);
            Some(Candidate {
                label: if stable {
                    format!("{version}（稳定）")
                } else {
                    version.clone()
                },
                version,
                stable,
            })
        })
        .collect();
    // 默认用第一个，所以稳定版排前面
    out.sort_by(|a, b| b.stable.cmp(&a.stable));
    Ok(out)
}

/// 拉 profile JSON 落盘，然后用原版安装流程补齐依赖库。
pub async fn install_meta(
    client: &Client,
    paths: &Paths,
    mc: &str,
    kind: LoaderKind,
    loader_version: &str,
    progress: Option<Progress>,
) -> Result<String, String> {
    let base = match kind {
        LoaderKind::Fabric => FABRIC_META,
        LoaderKind::Quilt => QUILT_META,
        _ => return Err("这个加载器不走 meta 接口".into()),
    };
    let url = format!("{base}/versions/loader/{mc}/{loader_version}/profile/json");
    let body = get_text(client, &url).await?;
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("解析 profile 失败: {e}"))?;
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("profile 里没有 id")?
        .to_string();

    let dest = paths.version_json(&id);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    std::fs::write(&dest, &body).map_err(|e| format!("写入 {} 失败: {e}", dest.display()))?;

    install::install(
        client,
        paths,
        &id,
        progress.unwrap_or_else(Progress::none),
    )
    .await?;

    Ok(id)
}

// ---------------- Forge / NeoForge ----------------


fn version_ids(paths: &Paths) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_loader_kind() {
        assert_eq!(LoaderKind::parse("Fabric"), Some(LoaderKind::Fabric));
        assert_eq!(LoaderKind::parse("neoforge"), Some(LoaderKind::NeoForge));
        assert_eq!(LoaderKind::parse("vanilla"), None);
    }
}
