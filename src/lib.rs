//! Dream Launchpad Launcher 核心逻辑。
//!
//! 约定：这个 crate **不依赖任何 GUI / WebView 库**，所有能力都能用单元测试验证。
//! 界面层（`src-tauri`）只负责窗口、命令转发和状态展示。
//!
//! 模块划分（按「干什么」分组，每组一个目录）：
//!
//! - [`store`]     本地数据：数据目录、设置、账号、登录
//! - [`mc`]        Minecraft 数据模型：清单、版本 JSON、Maven 坐标、规则、扫描
//! - [`net`]       网络：并行下载、镜像重写
//! - [`install`]   安装编排：依赖库 / natives / 资源
//! - [`launch`]    启动参数构建与进程拉起
//! - [`loader`]    模组加载器（Fabric / Forge / NeoForge / Quilt）与 OptiFine
//! - [`market`]    资源市场（Modrinth / CurseForge）
//! - [`content`]   内容管理：模组、存档、资源包、日志、崩溃
//! - [`sys`]       系统：硬件信息、Java 探测与下载

pub mod content;
pub mod install;
pub mod launch;
pub mod loader;
pub mod market;
pub mod minecraft;
pub mod net;
pub mod store;
pub mod sys;

// 兼容旧路径：`crate::paths::Paths` 这类写法继续可用，调用点不用改
pub use store::{accounts, auth, auth_msa, paths, settings};
/// 兼容旧路径：模组中文名与中文搜索词
pub use market::zh as market_zh;
pub use sys::{java, runtime, system};

pub use paths::Paths;

/// 核心库版本（跟随 Cargo.toml）。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
