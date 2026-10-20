//! Dream Launchpad Launcher 核心逻辑。
//!
//! 约定：这个 crate **不依赖任何 GUI / WebView 库**，所有能力都能用单元测试验证。
//! 界面层（`src-tauri`）只负责窗口、命令转发和状态展示。

pub mod infra;

pub use infra::{default_game_dir, instance_dir};

/// 核心库版本（跟随 Cargo.toml）。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
