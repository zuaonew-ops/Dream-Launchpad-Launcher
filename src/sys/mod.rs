//! 系统相关：硬件信息、Java 探测与下载。

pub mod java;
pub mod runtime;
pub mod system;

pub use system::info as system_info;
