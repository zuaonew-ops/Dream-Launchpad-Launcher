//! 并行下载 + SHA1 校验。

use futures_util::stream::{self, StreamExt};
use reqwest::Client;
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use reqwest::header::{CONTENT_RANGE, RANGE};
use reqwest::StatusCode;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

#[derive(Clone, Debug)]
pub struct Job {
    pub url: String,
    /// 主地址失败后的回退地址（通常是官方源）
    pub fallback: Option<String>,
    pub dest: PathBuf,
    pub sha1: Option<String>,
}

impl Job {
    pub fn new(url: impl Into<String>, dest: impl Into<PathBuf>, sha1: Option<String>) -> Self {
        Self {
            url: url.into(),
            fallback: None,
            dest: dest.into(),
            sha1,
        }
    }

    /// 官方 URL + 可选镜像：镜像优先，官方兜底。
    pub fn mirrored(
        url: impl Into<String>,
        dest: impl Into<PathBuf>,
        sha1: Option<String>,
        version_id: Option<&str>,
    ) -> Self {
        let official = url.into();
        match crate::net::mirror::rewrite(&official, version_id) {
            Some(primary) => Self {
                url: primary,
                fallback: Some(official),
                dest: dest.into(),
                sha1,
            },
            None => Self::new(official, dest, sha1),
        }
    }
}

/// 进度回调：(阶段名, 已完成, 总数)
#[derive(Clone)]
pub struct Progress(Arc<dyn Fn(&'static str, usize, usize) + Send + Sync>);

impl Progress {
    pub fn new(f: impl Fn(&'static str, usize, usize) + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }

    pub fn none() -> Self {
        Self(Arc::new(|_, _, _| {}))
    }

    pub fn call(&self, stage: &'static str, done: usize, total: usize) {
        (self.0)(stage, done, total);
    }
}

/// 下载一批文件；已存在且校验通过的直接跳过。
pub async fn run(
    client: &Client,
    jobs: Vec<Job>,
    concurrency: usize,
    stage: &'static str,
    progress: &Progress,
) -> Result<(), String> {
    let total = jobs.len();
    let done = Arc::new(AtomicUsize::new(0));
    progress.call(stage, 0, total);

    let results: Vec<Result<(), String>> = stream::iter(jobs)
        .map(|job| {
            let client = client.clone();
            let done = done.clone();
            async move {
                let result = fetch_one(&client, &job).await;
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                if n == 1 || n % 8 == 0 || n == total {
                    progress.call(stage, n, total);
                }
                result
            }
        })
        .buffer_unordered(concurrency.max(1))
        .collect()
        .await;

    progress.call(stage, total, total);
    results.into_iter().collect::<Result<Vec<_>, _>>().map(|_| ())
}

async fn fetch_one(client: &Client, job: &Job) -> Result<(), String> {
    if let Some(expected) = &job.sha1 {
        if file_matches(&job.dest, expected).await {
            return Ok(());
        }
    } else if job.dest.is_file() {
        return Ok(());
    }

    if let Some(parent) = job.dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("创建目录 {} 失败: {e}", parent.display()))?;
    }

    let mut last_error = String::new();
    for attempt in 1..=3 {
        // 前两次走主地址（可能是镜像），最后一次换回退地址（官方）
        let url = match (&job.fallback, attempt) {
            (Some(fallback), 3) => fallback.as_str(),
            _ => job.url.as_str(),
        };
        match download_to(client, url, &job.dest).await {
            Ok(()) => {
                if let Some(expected) = &job.sha1 {
                    if !file_matches(&job.dest, expected).await {
                        last_error = format!("{} 校验不通过", job.url);
                        let _ = tokio::fs::remove_file(&job.dest).await;
                        continue;
                    }
                }
                return Ok(());
            }
            Err(e) => {
                last_error = format!("第 {attempt} 次下载 {url} 失败: {e}");
                let _ = tokio::fs::remove_file(&job.dest).await;
            }
        }
    }
    Err(last_error)
}

/// 超过这个大小才考虑分片（小文件分片反而更慢）
const MULTIPART_THRESHOLD: u64 = 4 * 1024 * 1024;
/// 分片数：再多会被镜像限速，收益不明显
const MULTIPART_PARTS: usize = 4;

/// 下载一个文件：大文件且服务端支持 Range 时走分片多连接，否则单连接。
async fn download_to(client: &Client, url: &str, dest: &Path) -> Result<(), String> {
    if let Some(size) = probe_size(client, url).await {
        if size >= MULTIPART_THRESHOLD {
            if std::env::var_os("DLP_DEBUG_DOWNLOAD").is_some() {
                eprintln!("[download] 分片 {MULTIPART_PARTS} 连接 · {size} 字节 · {url}");
            }
            if download_multipart(client, url, dest, size).await.is_ok() {
                return Ok(());
            }
            if std::env::var_os("DLP_DEBUG_DOWNLOAD").is_some() {
                eprintln!("[download] 分片失败，退回单连接 · {url}");
            }
            // 分片失败（镜像限流 / 中途断开）就退回单连接，别让整个安装挂掉
            let _ = tokio::fs::remove_file(dest.with_extension("part")).await;
        }
    }
    download_single(client, url, dest).await
}

/// 探测文件大小：用 Range 请求首字节，206 且有 Content-Range 才算支持分片。
async fn probe_size(client: &Client, url: &str) -> Option<u64> {
    let response = client
        .get(url)
        .header(RANGE, "bytes=0-0")
        .send()
        .await
        .ok()?;
    if response.status() != StatusCode::PARTIAL_CONTENT {
        return None;
    }
    let value = response.headers().get(CONTENT_RANGE)?.to_str().ok()?;
    // 形如 "bytes 0-0/1234567"
    let total = value.rsplit('/').next()?.trim().parse::<u64>().ok()?;
    (total > 0).then_some(total)
}

async fn download_single(client: &Client, url: &str, dest: &Path) -> Result<(), String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let tmp = dest.with_extension("part");
    tokio::fs::write(&tmp, &bytes)
        .await
        .map_err(|e| format!("写入 {} 失败: {e}", tmp.display()))?;
    tokio::fs::rename(&tmp, dest)
        .await
        .map_err(|e| format!("重命名到 {} 失败: {e}", dest.display()))?;
    Ok(())
}

/// 分片下载：每个分片一个 Range 请求，并发写入同一文件的对应偏移。
async fn download_multipart(
    client: &Client,
    url: &str,
    dest: &Path,
    size: u64,
) -> Result<(), String> {
    let tmp = dest.with_extension("part");
    let file = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("创建 {} 失败: {e}", tmp.display()))?;
    file.set_len(size)
        .await
        .map_err(|e| format!("预分配 {} 失败: {e}", tmp.display()))?;
    drop(file);

    let parts = MULTIPART_PARTS.min((size / 1024 / 1024).max(1) as usize);
    let chunk = size.div_ceil(parts as u64);

    let results: Vec<Result<(), String>> = stream::iter(0..parts)
        .map(|index| {
            let client = client.clone();
            let tmp = tmp.clone();
            let start = chunk * index as u64;
            let end = ((start + chunk).min(size)).saturating_sub(1);
            async move { fetch_range(&client, url, &tmp, start, end).await }
        })
        .buffer_unordered(parts)
        .collect()
        .await;

    for result in results {
        result?;
    }

    tokio::fs::rename(&tmp, dest)
        .await
        .map_err(|e| format!("重命名到 {} 失败: {e}", dest.display()))?;
    Ok(())
}

async fn fetch_range(
    client: &Client,
    url: &str,
    tmp: &Path,
    start: u64,
    end: u64,
) -> Result<(), String> {
    let response = client
        .get(url)
        .header(RANGE, format!("bytes={start}-{end}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status() != StatusCode::PARTIAL_CONTENT {
        return Err(format!("{url} 不支持分片（HTTP {}）", response.status()));
    }

    let mut writer = tokio::fs::OpenOptions::new()
        .write(true)
        .open(tmp)
        .await
        .map_err(|e| format!("打开 {} 失败: {e}", tmp.display()))?;
    writer
        .seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|e| e.to_string())?;

    let mut written = 0u64;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        writer
            .write_all(&chunk)
            .await
            .map_err(|e| format!("写入分片失败: {e}"))?;
        written += chunk.len() as u64;
    }
    writer.flush().await.map_err(|e| e.to_string())?;
    drop(writer);

    let expected = end - start + 1;
    if written != expected {
        return Err(format!("分片 {start}-{end} 只有 {written} 字节，应为 {expected}"));
    }
    Ok(())
}

/// 文件是否存在且 SHA1 匹配。
pub async fn file_matches(path: &Path, expected: &str) -> bool {
    let Ok(mut file) = tokio::fs::File::open(path).await else {
        return false;
    };
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        match file.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(_) => return false,
        }
    }
    hex::encode(hasher.finalize()).eq_ignore_ascii_case(expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn matches_known_content() {
        let dir = std::env::temp_dir().join("dlp-download-test");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let file = dir.join("a.txt");
        tokio::fs::write(&file, b"abc").await.unwrap();
        // sha1("abc")
        assert!(file_matches(&file, "a9993e364706816aba3e25717850c26c9cd0d89d").await);
        assert!(!file_matches(&file, "deadbeef").await);
    }
}
