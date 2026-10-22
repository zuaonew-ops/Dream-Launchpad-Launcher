//! 网络：并行下载、完整性校验、下载源镜像。

pub mod download;
pub mod fetch;
pub mod mirror;

pub use download::{human, Job, Progress};
pub use fetch::{file_matches, sha1_file};
