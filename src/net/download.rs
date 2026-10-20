//! 并行下载 + SHA1 校验。

use futures_util::stream::{self, StreamExt};
use reqwest::Client;
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::AsyncReadExt;

#[derive(Clone, Debug)]
pub struct Job {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
}

impl Job {
    pub fn new(url: impl Into<String>, dest: impl Into<PathBuf>, sha1: Option<String>) -> Self {
        Self {
            url: url.into(),
            dest: dest.into(),
            sha1,
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
        match download_to(client, &job.url, &job.dest).await {
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
                last_error = format!("第 {attempt} 次下载 {} 失败: {e}", job.url);
                let _ = tokio::fs::remove_file(&job.dest).await;
            }
        }
    }
    Err(last_error)
}

async fn download_to(client: &Client, url: &str, dest: &Path) -> Result<(), String> {
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
