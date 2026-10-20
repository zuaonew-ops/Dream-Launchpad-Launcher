//! 安装一个版本：元数据 → 依赖库 → natives → 资源文件。

pub mod assets;
pub mod natives;

use crate::minecraft::{load_version, manifest::VersionEntry, platform, version::Artifact, VersionJson};
use crate::net::{download, Job, Progress};
use crate::paths::Paths;
use reqwest::Client;

const CONCURRENCY: usize = 16;

/// 确保 `versions/<id>/<id>.json` 存在（不存在就按清单里的 URL 拉）。
pub async fn ensure_version_json(
    client: &Client,
    paths: &Paths,
    entry: &VersionEntry,
) -> Result<(), String> {
    let dest = paths.version_json(&entry.id);
    let job = Job::mirrored(entry.url.clone(), dest, entry.sha1.clone(), Some(&entry.id));
    download::run(client, vec![job], 1, "版本信息", &Progress::none()).await
}

/// 安装一个版本（幂等：已存在且校验通过的文件会跳过）。
pub async fn install(
    client: &Client,
    paths: &Paths,
    id: &str,
    progress: Progress,
) -> Result<(), String> {
    let version = load_version(paths, id)?;

    let mut jobs = metadata_jobs(paths, id, &version);
    let (lib_jobs, native_jobs) = libraries_jobs(paths, &version);
    let native_count = native_jobs.len();
    jobs.extend(lib_jobs);
    jobs.extend(native_jobs);

    download::run(client, jobs, CONCURRENCY, "下载游戏文件", &progress).await?;
    natives::extract_all(paths, id, &version, &progress, native_count)?;

    if let Some(index) = &version.asset_index {
        let index_file = paths.indexes().join(format!("{}.json", index.id));
        let parsed = assets::load_index(&index_file)?;
        assets::download_objects(client, paths, &parsed, &progress).await?;
    }

    Ok(())
}

/// 客户端 jar + 日志配置 + 资源索引。
fn metadata_jobs(paths: &Paths, id: &str, version: &VersionJson) -> Vec<Job> {
    let mut jobs = Vec::new();

    if let Some(client_jar) = version.downloads.as_ref().and_then(|d| d.client.as_ref()) {
        jobs.push(Job::mirrored(
            client_jar.url.clone(),
            paths.client_jar(id),
            client_jar.sha1.clone(),
            Some(id),
        ));
    }

    if let Some(file) = log_config_file(version) {
        let name = file
            .path
            .clone()
            .unwrap_or_else(|| "client-log.xml".to_string());
        jobs.push(Job::new(
            file.url.clone(),
            paths.log_configs().join(name),
            file.sha1.clone(),
        ));
    }

    if let Some(index) = &version.asset_index {
        jobs.push(Job::new(
            index.url.clone(),
            paths.indexes().join(format!("{}.json", index.id)),
            index.sha1.clone(),
        ));
    }

    jobs
}

pub(crate) fn log_config_file(version: &VersionJson) -> Option<&Artifact> {
    version
        .logging
        .as_ref()
        .and_then(|l| l.client.as_ref())
        .and_then(|c| c.file.as_ref())
}

/// 依赖库（含 natives 分类器）。
fn libraries_jobs(paths: &Paths, version: &VersionJson) -> (Vec<Job>, Vec<Job>) {
    let mut libs = Vec::new();
    let mut natives = Vec::new();

    for lib in version.active_libraries() {
        let Some(downloads) = &lib.downloads else {
            continue;
        };
        if let Some(artifact) = &downloads.artifact {
            if let Some(path) = &artifact.path {
                // 非本平台的 natives jar 不下载（rules 只按系统筛，不按架构）
                if !platform::is_usable(path) {
                    continue;
                }
                libs.push(Job::mirrored(
                    artifact.url.clone(),
                    paths.libraries().join(path),
                    artifact.sha1.clone(),
                    None,
                ));
            }
        }
        if let Some(key) = version.native_key(lib) {
            if let Some(native) = downloads.classifiers.get(&key) {
                if let Some(path) = &native.path {
                    natives.push(Job::mirrored(
                        native.url.clone(),
                        paths.libraries().join(path),
                        native.sha1.clone(),
                        None,
                    ));
                }
            }
        }
    }

    (libs, natives)
}
