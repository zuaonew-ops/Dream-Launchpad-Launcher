//! 单个文件的下载：单连接 / 分片 / 校验。

use super::download::{Detail, Job};
use futures_util::stream::{self, StreamExt};
use reqwest::header::{CONTENT_RANGE, RANGE};
use reqwest::{Client, StatusCode};
use sha1::{Digest, Sha1};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
pub(crate) async fn fetch_one(client: &Client, job: &Job, detail: &Detail) -> Result<(), String> {
    // 缓存命中也要计入字节：否则进度条按字节算时会一直停在 0%
    if let Some(expected) = &job.sha1 {
        if file_matches(&job.dest, expected).await {
            detail.add(job.size);
            return Ok(());
        }
    } else if job.dest.is_file() {
        detail.add(job.size);
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
        match download_to(client, url, &job.dest, detail).await {
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
                // 退避一下再重试：镜像限速时立刻重发只会更慢
                tokio::time::sleep(std::time::Duration::from_millis(250 * attempt as u64))
                    .await;
            }
        }
    }
    Err(last_error)
}

/// 超过这个大小才考虑分片（小文件分片反而更慢）
const MULTIPART_THRESHOLD: u64 = 4 * 1024 * 1024;
/// 分片数：再多会被镜像限速，收益不明显
const MULTIPART_PARTS: usize = 6;

/// 下载一个文件：大文件且服务端支持 Range 时走分片多连接，否则单连接。
pub(crate) async fn download_to(client: &Client, url: &str, dest: &Path, detail: &Detail) -> Result<(), String> {
    if let Some(size) = probe_size(client, url).await {
        if size >= MULTIPART_THRESHOLD {
            if std::env::var_os("DLP_DEBUG_DOWNLOAD").is_some() {
                eprintln!("[download] 分片 {MULTIPART_PARTS} 连接 · {size} 字节 · {url}");
            }
            if download_multipart(client, url, dest, size, detail).await.is_ok() {
                return Ok(());
            }
            if std::env::var_os("DLP_DEBUG_DOWNLOAD").is_some() {
                eprintln!("[download] 分片失败，退回单连接 · {url}");
            }
            // 分片失败（镜像限流 / 中途断开）就退回单连接，别让整个安装挂掉
            let _ = tokio::fs::remove_file(dest.with_extension("part")).await;
        }
    }
    download_single(client, url, dest, detail).await
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

async fn download_single(
    client: &Client,
    url: &str,
    dest: &Path,
    detail: &Detail,
) -> Result<(), String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let tmp = dest.with_extension("part");
    let mut file = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("创建 {} 失败: {e}", tmp.display()))?;

    // 边下边写边计数：进度条才会平滑，不会「下完才跳一格」
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("写入 {} 失败: {e}", tmp.display()))?;
        detail.add(chunk.len() as u64);
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);

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
    detail: &Detail,
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
            async move { fetch_range(&client, url, &tmp, start, end, detail).await }
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
    detail: &Detail,
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
        detail.add(chunk.len() as u64);
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
