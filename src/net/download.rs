//! 并行下载 + SHA1 校验。

use futures_util::stream::{self, StreamExt};
use reqwest::Client;
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
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
    /// 字节数（未知为 0）—— 用来算按字节的进度
    pub size: u64,
}

impl Job {
    pub fn new(url: impl Into<String>, dest: impl Into<PathBuf>, sha1: Option<String>) -> Self {
        Self {
            url: url.into(),
            fallback: None,
            dest: dest.into(),
            sha1,
            size: 0,
        }
    }

    /// 带上字节数，进度条就能按字节平滑推进
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
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
                size: 0,
            },
            None => Self::new(official, dest, sha1),
        }
    }
}

/// 下载中的实时细节：已收字节、总字节、当前文件名。
///
/// 这些不进回调参数，而是放在共享结构里 —— 回调每 200ms 被调一次，
/// 界面直接读它就能显示「正在下载 xxx / 3.2 MB/s」。
#[derive(Default)]
pub struct Detail {
    done: AtomicU64,
    total: AtomicU64,
    current: Mutex<String>,
    speed: Mutex<String>,
    /// 最近几秒的采样 (时间, 已收字节)，用来算**瞬时**速度
    samples: Mutex<std::collections::VecDeque<(std::time::Instant, u64)>>,
}

impl Detail {
    pub fn reset(&self, total: u64) {
        self.done.store(0, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
        if let Ok(mut samples) = self.samples.lock() {
            samples.clear();
        }
    }

    pub fn add(&self, bytes: u64) {
        let total = self.done.fetch_add(bytes, Ordering::Relaxed) + bytes;
        // 每 250ms 采一个点，只留最近 3 秒 —— 平均速度会因为文件大小不均而跳，
        // 瞬时速度才看得出「现在快不快」。
        let mut samples = match self.samples.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let now = std::time::Instant::now();
        let stale = samples
            .front()
            .map(|(at, _)| now.duration_since(*at) > std::time::Duration::from_secs(3))
            .unwrap_or(true);
        let due = samples
            .back()
            .map(|(at, _)| now.duration_since(*at).as_millis() >= 250)
            .unwrap_or(true);
        if due || stale {
            samples.push_back((now, total));
        }
        while samples
            .front()
            .map(|(at, _)| now.duration_since(*at) > std::time::Duration::from_secs(3))
            .unwrap_or(false)
        {
            samples.pop_front();
        }
    }

    /// 最近 3 秒的平均速率（就是用户眼里的「当前速度」）
    pub fn instant_speed(&self) -> String {
        let samples = match self.samples.lock() {
            Ok(guard) => guard,
            Err(_) => return String::new(),
        };
        let (Some((first_at, first_bytes)), Some((last_at, last_bytes))) =
            (samples.front(), samples.back())
        else {
            return String::new();
        };
        let seconds = last_at.duration_since(*first_at).as_secs_f64();
        if seconds < 0.2 || last_bytes <= first_bytes {
            return String::new();
        }
        format!("{}/s", human((last_bytes - first_bytes) as f64 / seconds))
    }

    pub fn set_current(&self, name: &str) {
        if let Ok(mut slot) = self.current.lock() {
            slot.clear();
            slot.push_str(name);
        }
    }

    pub fn done(&self) -> u64 {
        self.done.load(Ordering::Relaxed)
    }

    pub fn total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }

    pub fn current(&self) -> String {
        self.current.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn set_speed(&self, value: &str) {
        if let Ok(mut slot) = self.speed.lock() {
            slot.clear();
            slot.push_str(value);
        }
    }

    pub fn speed(&self) -> String {
        self.speed.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

/// 进度里显示的短名字：取文件名，太长就截断
fn file_label(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if name.chars().count() > 48 {
        name.chars().take(45).collect::<String>() + "…"
    } else {
        name
    }
}

/// 人类可读的字节数
pub fn human(bytes: f64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// 进度回调：(阶段名, 已完成, 总数, 实时字节信息)
#[derive(Clone)]
pub struct Progress {
    sink: Arc<dyn Fn(&'static str, usize, usize, &Detail) + Send + Sync>,
    pub detail: Arc<Detail>,
}

impl Progress {
    pub fn new(f: impl Fn(&'static str, usize, usize, &Detail) + Send + Sync + 'static) -> Self {
        Self {
            sink: Arc::new(f),
            detail: Arc::new(Detail::default()),
        }
    }

    pub fn none() -> Self {
        Self::new(|_, _, _, _| {})
    }

    pub fn call(&self, stage: &'static str, done: usize, total: usize) {
        (self.sink)(stage, done, total, &self.detail);
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
    let bytes_total: u64 = jobs.iter().map(|job| job.size).sum();
    if bytes_total > 0 {
        progress.detail.reset(bytes_total);
    }
    progress.call(stage, 0, total);

    // 大文件和小文件分开跑：
    // - 一堆几 KB 的小文件，瓶颈在往返延迟，并发越高越划算
    // - 几个几十 MB 的大文件，并发太高会互相抢带宽，反而变慢
    // 混在一起跑就是「一会儿快一会儿慢」的来源。
    let (small, large): (Vec<Job>, Vec<Job>) = jobs
        .into_iter()
        .partition(|job| job.size < SMALL_FILE_BYTES || job.size == 0);

    let small_conc = (concurrency * 2).max(8);
    let large_conc = (concurrency / 2).max(4);

    let finished = Arc::new(AtomicUsize::new(0));
    let small_task = run_batch(client, small, small_conc, stage, progress, total, &finished);
    let large_task = run_batch(client, large, large_conc, stage, progress, total, &finished);
    let (a, b) = tokio::join!(small_task, large_task);
    a?;
    b?;

    progress.call(stage, total, total);
    Ok(())
}

/// 小于这个大小的算「小文件」，走高并发
const SMALL_FILE_BYTES: u64 = 512 * 1024;

/// 跑一批 job，共享同一个计数器
async fn run_batch(
    client: &Client,
    jobs: Vec<Job>,
    concurrency: usize,
    stage: &'static str,
    progress: &Progress,
    total_all: usize,
    finished: &Arc<AtomicUsize>,
) -> Result<(), String> {
    if jobs.is_empty() {
        return Ok(());
    }
    let bytes_done = Arc::new(AtomicU64::new(0));

    let results: Vec<Result<(), String>> = stream::iter(jobs)
        .map(|job| {
            let client = client.clone();
            let finished = finished.clone();
            let detail = progress.detail.clone();
            let progress = progress.clone();
            let bytes_done = bytes_done.clone();
            async move {
                detail.set_current(&file_label(&job.dest));
                let result = fetch_one(&client, &job, &detail).await;
                let n = finished.fetch_add(1, Ordering::Relaxed) + 1;
                bytes_done.fetch_add(job.size, Ordering::Relaxed);
                let speed = detail.instant_speed();
                detail.set_speed(&speed);
                if n == 1 || n % 4 == 0 || n == total_all {
                    progress.call(stage, n, total_all);
                }
                result
            }
        })
        .buffer_unordered(concurrency.max(1))
        .collect()
        .await;

    results.into_iter().collect::<Result<Vec<_>, _>>().map(|_| ())
}

async fn fetch_one(client: &Client, job: &Job, detail: &Detail) -> Result<(), String> {
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
async fn download_to(client: &Client, url: &str, dest: &Path, detail: &Detail) -> Result<(), String> {
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
