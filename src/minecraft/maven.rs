//! Maven 坐标解析。
//!
//! 原版清单里每个库都带 `downloads.artifact.path`，但 Fabric / Quilt 这类加载器
//! 的版本 JSON 只写 `name`（`group:artifact:version`）和 `url`（仓库地址），
//! 没有下载信息 —— 必须自己拼路径，否则这些库既不会下载也进不了 classpath。

/// 把 `org.ow2.asm:asm:9.10.1` 解析成 `org/ow2/asm/asm/9.10.1/asm-9.10.1.jar`
///
/// 支持带 classifier 的形式：`net.fabricmc:sponge-mixin:0.17.4+mixin.0.8.7`
pub fn artifact_path(name: &str) -> Option<String> {
    let mut parts = name.split(':');
    let group = parts.next()?.trim();
    let artifact = parts.next()?.trim();
    let version = parts.next()?.trim();
    if group.is_empty() || artifact.is_empty() || version.is_empty() {
        return None;
    }
    let classifier = parts.next().map(str::trim).filter(|c| !c.is_empty());
    let group_path = group.replace('.', "/");
    let file = match classifier {
        Some(classifier) => format!("{artifact}-{version}-{classifier}.jar"),
        None => format!("{artifact}-{version}.jar"),
    };
    Some(format!("{group_path}/{artifact}/{version}/{file}"))
}

/// 库的下载地址：优先用 JSON 里给的仓库，否则回退 Maven Central
pub fn download_url(repo: Option<&str>, path: &str) -> String {
    let base = repo
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .unwrap_or("https://repo1.maven.org/maven2/");
    let base = base.trim_end_matches('/');
    format!("{base}/{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_coordinates() {
        assert_eq!(
            artifact_path("org.ow2.asm:asm:9.10.1").as_deref(),
            Some("org/ow2/asm/asm/9.10.1/asm-9.10.1.jar")
        );
    }

    #[test]
    fn parses_version_with_plus() {
        assert_eq!(
            artifact_path("net.fabricmc:sponge-mixin:0.17.4+mixin.0.8.7").as_deref(),
            Some("net/fabricmc/sponge-mixin/0.17.4+mixin.0.8.7/sponge-mixin-0.17.4+mixin.0.8.7.jar")
        );
    }

    #[test]
    fn rejects_short_names() {
        assert_eq!(artifact_path("asm"), None);
    }

    #[test]
    fn builds_url() {
        assert_eq!(
            download_url(Some("https://maven.fabricmc.net/"), "a/b/1/b-1.jar"),
            "https://maven.fabricmc.net/a/b/1/b-1.jar"
        );
        assert_eq!(
            download_url(None, "a/b/1/b-1.jar"),
            "https://repo1.maven.org/maven2/a/b/1/b-1.jar"
        );
    }
}
