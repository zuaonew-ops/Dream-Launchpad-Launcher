//! 下载源映射：把 Mojang 官方地址改写到镜像（BMCLAPI），失败时回退官方。
//!
//! 只镜像"大文件"（客户端 jar、依赖库、资源对象）和清单/版本 JSON；
//! 资源索引、日志配置这类几 KB 的元数据继续走官方，避免镜像缺项。

pub const BMCLAPI: &str = "https://bmclapi2.bangbang93.com";

/// 官方清单地址。
pub const OFFICIAL_MANIFEST: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

/// 把一个官方 URL 映射到 BMCLAPI；没有对应规则时返回 `None`。
pub fn rewrite(url: &str, version_id: Option<&str>) -> Option<String> {
    if let Some(rest) = url.strip_prefix("https://libraries.minecraft.net/") {
        return Some(format!("{BMCLAPI}/maven/{rest}"));
    }
    if let Some(rest) = url.strip_prefix("https://resources.download.minecraft.net/") {
        return Some(format!("{BMCLAPI}/assets/{rest}"));
    }
    if url == OFFICIAL_MANIFEST {
        return Some(format!("{BMCLAPI}/mc/game/version_manifest_v2.json"));
    }
    if let Some(id) = version_id {
        if url.ends_with("/client.jar") {
            return Some(format!("{BMCLAPI}/version/{id}/client"));
        }
        if url.ends_with(&format!("/{id}.json")) {
            return Some(format!("{BMCLAPI}/version/{id}/json"));
        }
    }
    None
}

/// 资源对象地址（官方 + 镜像）。
pub fn asset_urls(hash: &str) -> (String, String) {
    let prefix = &hash[0..2];
    (
        format!("{BMCLAPI}/assets/{prefix}/{hash}"),
        format!("https://resources.download.minecraft.net/{prefix}/{hash}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_libraries() {
        assert_eq!(
            rewrite("https://libraries.minecraft.net/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar", None),
            Some(format!("{BMCLAPI}/maven/org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar"))
        );
    }

    #[test]
    fn rewrites_client_and_version_json() {
        assert_eq!(
            rewrite("https://piston-data.mojang.com/v1/objects/abc/client.jar", Some("1.20.1")),
            Some(format!("{BMCLAPI}/version/1.20.1/client"))
        );
        assert_eq!(
            rewrite("https://piston-meta.mojang.com/v1/packages/abc/1.20.1.json", Some("1.20.1")),
            Some(format!("{BMCLAPI}/version/1.20.1/json"))
        );
    }

    #[test]
    fn leaves_unknown_urls_alone() {
        assert_eq!(
            rewrite("https://piston-meta.mojang.com/v1/packages/abc/5.json", Some("1.20.1")),
            None
        );
    }

    #[test]
    fn asset_urls_pair_up() {
        let (mirror, official) = asset_urls("abcdef");
        assert!(mirror.starts_with(BMCLAPI));
        assert!(official.starts_with("https://resources.download.minecraft.net/ab/"));
    }
}
