//! 模组加载器与 OptiFine 安装。
//!
//! 两条路线：
//! - **Fabric / Quilt**：官方 meta 直接给 profile JSON，落盘后复用原版安装逻辑，不需要跑安装器
//! - **Forge / NeoForge**：官方 installer 支持 `--installClient`，用 Java 静默跑
//! - **OptiFine**：安装器只认 `~/.minecraft`，所以给它一个临时 home，装完把产物搬回来

use crate::install;
use crate::net::{download, Job, Progress};
use crate::paths::Paths;
use reqwest::Client;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BMCLAPI: &str = "https://bmclapi2.bangbang93.com";
const FABRIC_META: &str = "https://meta.fabricmc.net/v2";
const QUILT_META: &str = "https://meta.quiltmc.org/v3";

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

pub async fn optifine_candidates(client: &Client, mc: &str) -> Result<Vec<Candidate>, String> {
    let url = format!("{BMCLAPI}/optifine/{mc}");
    let body = get_text(client, &url).await?;
    let list: Vec<serde_json::Value> =
        serde_json::from_str(&body).map_err(|e| format!("解析 {url} 失败: {e}"))?;

    let mut out: Vec<Candidate> = list
        .iter()
        .filter_map(|item| {
            let kind = item.get("type")?.as_str()?;
            let patch = item.get("patch")?.as_str()?;
            // 预览版 patch 形如 pre4，正式版是 I6 / G8
            let stable = !patch.starts_with("pre");
            Some(Candidate {
                version: format!("{kind}_{patch}"),
                label: if stable {
                    format!("{kind}_{patch}")
                } else {
                    format!("{kind}_{patch}（预览）")
                },
                stable,
            })
        })
        .collect();
    out.reverse();
    // 预览版（pre）放最后，默认取第一个
    out.sort_by(|a, b| b.stable.cmp(&a.stable));
    Ok(out)
}

/// 下载 OptiFine 并静默安装：给安装器一个临时 `~/.minecraft`，装完把产物搬回来。
pub async fn install_optifine(
    client: &Client,
    paths: &Paths,
    mc: &str,
    edition: &str,
    java: &Path,
    progress: Option<Progress>,
) -> Result<String, String> {
    // edition 形如 HD_U_I6 → BMCLAPI 路径是 /optifine/<mc>/HD_U/I6
    let (kind, patch) = edition
        .rsplit_once('_')
        .ok_or_else(|| format!("OptiFine 版本格式不对: {edition}"))?;

    let work = paths.root.join("dlp").join("optifine");
    std::fs::create_dir_all(&work).map_err(|e| format!("创建目录失败: {e}"))?;
    let jar = work.join(format!("OptiFine_{mc}_{edition}.jar"));
    if !jar.is_file() {
        let url = format!("{BMCLAPI}/optifine/{mc}/{kind}/{patch}");
        download::run(
            client,
            vec![Job::new(url, &jar, None)],
            1,
            "下载 OptiFine",
            &progress.clone().unwrap_or_else(Progress::none),
        )
        .await?;
    }

    // 临时 home：安装器只认 ~/.minecraft，把目标版本复制进去让它改
    let home = work.join("home");
    let sandbox = home.join(".minecraft");
    let _ = std::fs::remove_dir_all(&home);
    let version_dir = sandbox.join("versions").join(mc);
    std::fs::create_dir_all(&version_dir).map_err(|e| format!("创建沙箱目录失败: {e}"))?;
    for name in [format!("{mc}.json"), format!("{mc}.jar")] {
        let from = paths.version_dir(mc).join(&name);
        if from.is_file() {
            std::fs::copy(&from, version_dir.join(&name))
                .map_err(|e| format!("复制 {name} 到沙箱失败: {e}"))?;
        }
    }

    // OptiFine 安装器在 Windows 上用 %APPDATA%\\.minecraft 定位游戏目录，
    // 只改 user.home 没用（实测会被忽略），所以这里重定向 APPDATA。
    let (ok, output) = run_java_raw(
        java,
        &[
            format!("-Duser.home={}", home.display()),
            // main() 是 GUI 入口，装完会弹对话框；headless 下抛异常但不影响已写好的文件
            "-Djava.awt.headless=true".into(),
            "-cp".into(),
            jar.to_string_lossy().to_string(),
            "optifine.Installer".into(),
        ],
        &home,
        &[("APPDATA", home.to_string_lossy().to_string())],
    )?;

    let installed: Vec<String> = std::fs::read_dir(sandbox.join("versions"))
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();

    let id = installed
        .iter()
        .find(|id| id.contains("OptiFine"))
        .cloned()
        .ok_or_else(|| {
            format!(
                "OptiFine 安装器没有生成版本（退出码{}）。输出：\n{}",
                if ok { " 0" } else { " 非 0" },
                tail(&output, 1500)
            )
        })?;

    copy_dir(&sandbox.join("versions").join(&id), &paths.version_dir(&id))?;
    copy_dir(&sandbox.join("libraries"), &paths.libraries())?;
    let _ = std::fs::remove_dir_all(&home);

    Ok(id)
}

// ---------------- 工具 ----------------

async fn get_text(client: &Client, url: &str) -> Result<String, String> {
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
fn run_java(java: &Path, args: &[String], cwd: &Path) -> Result<String, String> {
    let (ok, text) = run_java_raw(java, args, cwd, &[])?;
    if ok {
        Ok(text)
    } else {
        Err(format!("安装器失败：\n{}", tail(&text, 1500)))
    }
}

fn run_java_raw(
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

fn tail(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_string();
    }
    chars[chars.len() - max..].iter().collect()
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
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
    fn parse_loader_kind() {
        assert_eq!(LoaderKind::parse("Fabric"), Some(LoaderKind::Fabric));
        assert_eq!(LoaderKind::parse("neoforge"), Some(LoaderKind::NeoForge));
        assert_eq!(LoaderKind::parse("vanilla"), None);
    }

    #[test]
    fn installer_urls_match_maven_layout() {
        assert!(installer_url(LoaderKind::Forge, "1.20.1", "47.2.0")
            .contains("forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-installer.jar"));
        assert!(installer_url(LoaderKind::NeoForge, "1.21.1", "21.1.72")
            .contains("neoforge/21.1.72/neoforge-21.1.72-installer.jar"));
    }

    #[test]
    fn tail_keeps_last_chars() {
        assert_eq!(tail("abcdef", 3), "def");
    }
}
