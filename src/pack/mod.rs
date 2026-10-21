//! 整合包：解析 `.mrpack`（Modrinth）与 CurseForge 的 zip，装成一个新版本。
//!
//! 两种格式的共同点：都是 zip，里面有一份「装什么」的清单 + 一份 `overrides/`
//! 直接覆盖到游戏目录。区别在清单格式和文件从哪来：
//! - Modrinth：清单自带每个文件的直链与哈希，不需要 API key
//! - CurseForge：清单只给 projectID/fileID，得用官方 API 换直链（需要 key）

pub mod curseforge;
pub mod install;
pub mod mrpack;

use serde::Serialize;

/// 整合包预览：装之前给用户看的摘要。
#[derive(Debug, Clone, Serialize)]
pub struct PackPreview {
    /// mrpack / curseforge
    pub format: String,
    pub name: String,
    pub version: String,
    /// 依赖的《我的世界》版本
    pub mc: String,
    /// 依赖的加载器（fabric-loader / forge / neoforge / quilt-loader），原版为 None
    pub loader: Option<(String, String)>,
    /// 要下载的文件数
    pub files: usize,
    /// 要下载的总字节数（CurseForge 清单不给大小，为 0）
    pub bytes: u64,
    /// 包里的 overrides 文件数
    pub overrides: usize,
    /// 清单里声明的文件（给界面列前几个看）
    pub sample: Vec<String>,
}
