//! 测试用临时目录：同一进程内每次调用都给一个新路径。
//!
//! 只用 pid 做名字会有坑 —— 并行跑的两个测试可能拿到同一个路径，
//! 互相删掉对方的文件，表现为「偶尔失败、重跑就好」。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn temp_dir(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "dlp-test-{tag}-{}-{n}",
        std::process::id()
    ));
    // 上一轮崩溃留下的残骸先清掉
    let _ = std::fs::remove_dir_all(&path);
    path
}
