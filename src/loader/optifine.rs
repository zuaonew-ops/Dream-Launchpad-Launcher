//! OptiFine：安装器只认默认 .minecraft，用临时 home 装完再搬回来。

use super::run::{copy_dir, get_text, run_java, run_java_raw, tail, version_ids};
use super::{Candidate, BMCLAPI};
use crate::net::{download, Job, Progress};
use crate::paths::Paths;
use reqwest::Client;
use std::path::Path;

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

