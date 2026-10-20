//! 网络层：并行下载、SHA1 校验、进度回调。

pub mod download;
pub mod mirror;

pub use download::{Job, Progress};
