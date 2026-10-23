//! 资源市场：Modrinth 与 CurseForge 两个来源。
//!
//! 文件按职责拆开，单文件不超过 ~200 行：
//! - `types.rs`：两边共用的类型（搜索结果、版本文件、项目详情）
//! - `modrinth.rs`：Modrinth 搜索 / 版本 / 详情
//! - `curseforge.rs`：CurseForge 搜索 / 版本 / 详情（需要 API key）
//! - `install.rs`：把选定文件下到版本目录
//!
//! 上层只跟 `Source` 打交道：`source.search(...)` / `source.versions(...)` / `source.detail(...)`。

mod curseforge;
mod install;
mod modrinth;
mod types;
pub mod updater;
pub mod zh;

pub use install::{install, install_file};
pub use updater::{identify, is_newer, latest, ModIdentity, ModUpdate};
pub use types::{FileChoice, Hit, ProjectDetail, SearchPage, VersionEntry};

use reqwest::Client;

/// 市场来源。CurseForge 官方 API 需要 key，没 key 时调用会得到可读错误。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Modrinth,
    CurseForge,
}

impl Source {
    pub fn parse(value: &str) -> Source {
        match value {
            "curseforge" => Source::CurseForge,
            _ => Source::Modrinth,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Source::Modrinth => "modrinth",
            Source::CurseForge => "curseforge",
        }
    }
}

/// CurseForge 的模组类型 id（官方 API 用）
pub(crate) fn curseforge_class(kind: &str) -> u32 {
    match kind {
        "modpack" => 4471,
        "resourcepack" => 12,
        "shader" => 6552,
        _ => 6,
    }
}

/// 资源种类 → 游戏目录下的子目录
pub(crate) fn target_dir(kind: &str) -> &'static str {
    match kind {
        "modpack" => "",
        "resourcepack" => "resourcepacks",
        "shader" => "shaderpacks",
        _ => "mods",
    }
}

pub(crate) fn project_type(kind: &str) -> &'static str {
    match kind {
        "resourcepack" => "resourcepack",
        "shader" => "shader",
        _ => "mod",
    }
}

pub struct Market {
    pub client: Client,
    /// CurseForge 的 API key；空串表示没配
    pub curseforge_key: String,
}

impl Market {
    pub fn new(client: Client, curseforge_key: String) -> Market {
        Market {
            client,
            curseforge_key,
        }
    }

    /// 搜索；`query` 为空时返回该来源的热门榜。
    pub async fn search(
        &self,
        source: Source,
        query: &str,
        kind: &str,
        mc: Option<&str>,
        loader: Option<&str>,
        category: Option<&str>,
        offset: u64,
        limit: u64,
    ) -> Result<SearchPage, String> {
        match source {
            Source::Modrinth => {
                modrinth::search(&self.client, query, kind, mc, loader, category, offset, limit).await
            }
            Source::CurseForge => {
                curseforge::search(
                    &self.client,
                    &self.curseforge_key,
                    query,
                    kind,
                    mc,
                    loader,
                    offset,
                    limit,
                )
                .await
            }
        }
    }

    /// 某项目在指定版本 + 加载器下的文件列表（按时间倒序）。
    pub async fn versions(
        &self,
        source: Source,
        project: &str,
        mc: Option<&str>,
        loader: Option<&str>,
    ) -> Result<Vec<VersionEntry>, String> {
        match source {
            Source::Modrinth => modrinth::versions(&self.client, project, mc, loader).await,
            Source::CurseForge => {
                curseforge::versions(&self.client, &self.curseforge_key, project, mc, loader).await
            }
        }
    }

    /// 项目详情（简介、图标、可用版本、MC 百科入口由前端按中文名判断）。
    pub async fn detail(&self, source: Source, project: &str) -> Result<ProjectDetail, String> {
        match source {
            Source::Modrinth => modrinth::detail(&self.client, project).await,
            Source::CurseForge => {
                curseforge::detail(&self.client, &self.curseforge_key, project).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_map_to_game_dirs() {
        assert_eq!(target_dir("mod"), "mods");
        assert_eq!(target_dir("resourcepack"), "resourcepacks");
        assert_eq!(target_dir("shader"), "shaderpacks");
        assert_eq!(project_type("shader"), "shader");
        assert_eq!(curseforge_class("mod"), 6);
    }

    #[test]
    fn source_parses() {
        assert_eq!(Source::parse("curseforge"), Source::CurseForge);
        assert_eq!(Source::parse("modrinth"), Source::Modrinth);
        assert_eq!(Source::parse("whatever"), Source::Modrinth);
    }
}
