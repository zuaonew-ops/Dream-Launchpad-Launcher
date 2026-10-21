//! 并行下载 + SHA1 校验。

use futures_util::stream::{self, StreamExt};
use reqwest::Client;
use std::path::{Path, PathBuf};

use super::fetch::fetch_one;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::sync::Arc;

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
pub(crate) fn file_label(path: &Path) -> String {
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
