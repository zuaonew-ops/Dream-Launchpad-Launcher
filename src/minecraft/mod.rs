//! Minecraft 数据模型：版本清单、版本 JSON、规则求值。

pub mod manifest;
pub mod platform;
pub mod rules;
pub mod version;

pub use manifest::{fetch, VersionEntry, VersionManifest};
pub use rules::allowed;
pub use version::{load as load_version, VersionJson};
