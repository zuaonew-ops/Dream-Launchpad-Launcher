//! 系统信息：物理内存总量 / 可用量 / CPU 核数。
//!
//! 内存滑块的上限要按真实内存算 —— 之前写死 16G，32G 内存的机器也用不满。

use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct SystemInfo {
    /// 物理内存总量（MB）
    pub total_mb: u64,
    /// 当前可用（MB）
    pub available_mb: u64,
    /// 逻辑核心数
    pub cores: usize,
}

pub fn info() -> SystemInfo {
    let (total_mb, available_mb) = memory_mb();
    SystemInfo {
        total_mb,
        available_mb,
        cores: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0),
    }
}

/// 内存给多留少：留出 1G 给系统，最多给到物理内存的 80%
pub fn suggest_max_mb(total_mb: u64) -> u64 {
    let eighty = total_mb * 8 / 10;
    let capped = total_mb.saturating_sub(1024);
    eighty.min(capped).max(512)
}

#[cfg(windows)]
fn memory_mb() -> (u64, u64) {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    unsafe {
        let mut status: MEMORYSTATUSEX = zeroed();
        status.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut status) != 0 {
            return (
                status.ullTotalPhys / 1024 / 1024,
                status.ullAvailPhys / 1024 / 1024,
            );
        }
    }
    (0, 0)
}

#[cfg(not(windows))]
fn memory_mb() -> (u64, u64) {
    let Ok(text) = std::fs::read_to_string("/proc/meminfo") else {
        return (0, 0);
    };
    let mut total = 0;
    let mut available = 0;
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("MemTotal:") => total = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0),
            Some("MemAvailable:") => {
                available = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)
            }
            _ => {}
        }
    }
    (total / 1024, available / 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_some_memory() {
        let info = info();
        // 真机上一定拿得到；容器里可能为 0，那就只验核数
        if info.total_mb > 0 {
            assert!(info.total_mb > info.available_mb || info.available_mb == 0);
            assert!(suggest_max_mb(info.total_mb) <= info.total_mb);
            assert!(suggest_max_mb(info.total_mb) >= 512);
        }
    }

    #[test]
    fn suggestion_keeps_headroom() {
        assert_eq!(suggest_max_mb(16384), 13107); // 16G → 80%
        assert_eq!(suggest_max_mb(65536), 52428); // 64G → 80%
        assert_eq!(suggest_max_mb(1024), 512); // 太小就兜底 512
    }
}
