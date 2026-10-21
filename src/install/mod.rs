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

    mark_dlp_installed(paths, id);

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

    // 加载器版本没有自己的 client jar，直接用父版本的
    if version.inherits_from.is_none() {
        if let Some(client_jar) = version.downloads.as_ref().and_then(|d| d.client.as_ref()) {
        jobs.push(Job::mirrored(
            client_jar.url.clone(),
            paths.client_jar(id),
            client_jar.sha1.clone(),
            Some(id),
        )
        .with_size(client_jar.size.unwrap_or(0)));
        }
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
        )
        .with_size(file.size.unwrap_or(0)));
    }

    if let Some(index) = &version.asset_index {
        jobs.push(Job::new(
            index.url.clone(),
            paths.indexes().join(format!("{}.json", index.id)),
            index.sha1.clone(),
        )
        .with_size(index.size.unwrap_or(0)));
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
        // 普通库：清单给路径就用清单的，Fabric 这类只给 Maven 坐标的现推
        if let Some(path) = lib.relative_path() {
            if platform::is_usable(&path) {
                if let Some(url) = lib.download_url() {
                    libs.push(
                        Job::mirrored(url, paths.libraries().join(&path), lib.sha1(), None)
                            .with_size(lib.size()),
                    );
                }
            }
        }
        // natives 分类器（老版本才用）
        if let Some(key) = version.native_key(lib) {
            if let Some(native) = lib
                .downloads
                .as_ref()
                .and_then(|d| d.classifiers.get(&key))
            {
                if let Some(path) = &native.path {
                    natives.push(
                        Job::mirrored(
                            native.url.clone(),
                            paths.libraries().join(path),
                            native.sha1.clone(),
                            None,
                        )
                        .with_size(native.size.unwrap_or(0)),
                    );
                }
            }
        }
    }

    (libs, natives)
}

/// 在版本目录里留一个标记，界面据此显示「DLP」徽章
pub fn mark_dlp_installed(paths: &Paths, id: &str) {
    let dir = paths.version_dir(id);
    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(
            dir.join(".dlp-installed"),
            env!("CARGO_PKG_VERSION").as_bytes(),
        );
    }
}

/// 这个版本是不是 DLP 装的
pub fn is_dlp_installed(paths: &Paths, id: &str) -> bool {
    paths.version_dir(id).join(".dlp-installed").is_file()
}
