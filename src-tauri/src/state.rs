//! 全局状态、跨命令共用的数据结构、以及进度事件。

use dlp_core::{net::Progress, paths::Paths};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

pub struct AppState {
    pub paths: Paths,
    pub client: reqwest::Client,
    /// 正在运行的游戏进程（退出后清空）
    pub game: Mutex<Option<RunningGame>>,
}

#[derive(Serialize)]
pub struct VersionSummary {
    pub id: String,
    pub kind: String,
    pub release_time: String,
    pub installed: bool,
    /// true = 只存在于本地 versions/ 目录（别的启动器装的 / 手动放的）
    pub local: bool,
    /// 愚人节快照（清单里仍标 release/snapshot，只能认 id）
    pub april_fools: bool,
    /// 从版本 id 推断的加载器（fabric / quilt / forge / neoforge），原版为 None
    pub loader: Option<String>,
    /// 这个版本是 DLP 装的（界面显示 DLP 徽章）
    pub dlp_installed: bool,
}

#[derive(Serialize, Clone)]
pub struct ProgressPayload {
    pub stage: String,
    pub done: usize,
    pub total: usize,
    /// 已收字节 / 总字节（0 = 这个阶段不按字节算）
    pub bytes_done: u64,
    pub bytes_total: u64,
    /// 当前正在处理的文件名
    pub current: String,
    /// 速度，形如 "3.2 MB/s"
    pub speed: String,
}

#[derive(Serialize, Clone)]
pub struct RunningGame {
    pub pid: u32,
    pub id: String,
}

#[derive(Serialize, Clone)]
pub struct GameExit {
    pub pid: u32,
    pub id: String,
    pub code: Option<i32>,
}

#[derive(Serialize)]
pub struct Environment {
    pub os: String,
    pub arch: String,
    pub tauri: String,
    pub core: String,
    pub data_dir: String,
}

/// 造一个把进度推到前端的回调。
///
/// 之前每个命令各写一遍同样的闭包（7 份），改字段要改 7 处 —— 现在只有这一份。
pub fn progress_to(app: AppHandle) -> Progress {
    Progress::new(move |stage, done, total, detail| {
        let _ = app.emit(
            "install-progress",
            ProgressPayload {
                stage: stage.to_string(),
                done,
                total,
                bytes_done: detail.done(),
                bytes_total: detail.total(),
                current: detail.current(),
                speed: detail.speed(),
            },
        );
    })
}
