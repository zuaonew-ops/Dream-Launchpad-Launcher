//! 两个来源共用的类型。
//!
//! 字段名和前端 types.ts 一一对应，改这里记得同步改前端。

use serde::Serialize;

/// 搜索结果里的一条
#[derive(Serialize, Clone, Debug)]
pub struct Hit {
    /// 来源内的项目 id（Modrinth 是 slug，CurseForge 是数字 id）
    pub id: String,
    pub slug: String,
    pub title: String,
    /// 中文名（词表命中才有，界面优先显示它）
    pub title_zh: Option<String>,
    pub description: String,
    pub downloads: u64,
    pub icon: Option<String>,
    /// "modrinth" / "curseforge"
    pub source: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct SearchPage {
    pub hits: Vec<Hit>,
    pub total: u64,
    pub offset: u64,
}

/// 一个版本下可直接下载的文件
#[derive(Serialize, Clone, Debug)]
pub struct FileChoice {
    pub version_id: String,
    /// 版本名，例如 "0.5.8+1.20.1"
    pub name: String,
    /// 展示用的版本号
    pub version_number: String,
    pub filename: String,
    pub download_url: String,
    pub downloads: u64,
    /// 这条文件支持的游戏版本
    pub game_versions: Vec<String>,
    /// 支持的加载器；空表示不限
    pub loaders: Vec<String>,
    /// 文件大小（字节）
    pub size: u64,
    /// 发布时间（RFC3339）
    pub date: String,
}

/// 详情页：项目本身 + 版本列表
#[derive(Serialize, Clone, Debug)]
pub struct ProjectDetail {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub title_zh: Option<String>,
    /// 纯文本简介
    pub description: String,
    /// 富文本正文（Modrinth 是 markdown，CurseForge 是 HTML；界面前端只做纯文本降级）
    pub body: String,
    pub icon: Option<String>,
    pub downloads: u64,
    pub categories: Vec<String>,
    /// 官网 / 项目页
    pub site_url: String,
    pub source: String,
}

/// 版本条目：一个版本可以挂多个文件
#[derive(Serialize, Clone, Debug)]
pub struct VersionEntry {
    pub version_id: String,
    pub name: String,
    pub version_number: String,
    pub downloads: u64,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub date: String,
    /// 这版的可下载文件（通常是 1 个主文件 + 若干附加）
    pub files: Vec<FileChoice>,
}
