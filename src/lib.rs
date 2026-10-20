//! Dream Launchpad Launcher 核心逻辑。
//!
//! 约定：这个 crate **不依赖任何 GUI / WebView 库**，所有能力都能用单元测试验证。
//! 界面层（`src-tauri`）只负责窗口、命令转发和状态展示。
//!
//! 模块划分：
//! - [`paths`]      数据目录布局
//! - [`auth`]       离线账号（UUID v3）
//! - [`java`]       Java 运行时探测
//! - [`minecraft`]  版本清单、版本 JSON（继承链）、规则求值
//! - [`net`]        并行下载与校验
//! - [`install`]    安装编排（元数据 / 依赖库 / natives / 资源）
//! - [`launch`]     启动参数构建与进程拉起

pub mod auth;
pub mod install;
pub mod java;
pub mod launch;
pub mod minecraft;
pub mod net;
pub mod paths;

pub use paths::Paths;

/// 核心库版本（跟随 Cargo.toml）。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
