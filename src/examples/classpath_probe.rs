//! 检查 classpath 里的文件是否都存在（不启动游戏）。
//!
//! 用法：`cargo run -p dlp-core --example classpath_probe -- fabric-loader-0.19.5-26.3`

use dlp_core::{minecraft, paths::Paths};
use std::path::Path;

fn main() -> Result<(), String> {
    let id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1.20.1".to_string());
    let root = std::env::var_os("DLP_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(Paths::default_root);
    let paths = Paths::new(root);
    let version = minecraft::load_version(&paths, &id)?;

    println!(
        "id={} inherits={:?} mainClass={:?} libs={} assets={:?}",
        version.id,
        version.inherits_from,
        version.main_class,
        version.libraries.len(),
        version.assets
    );

    let mut missing = 0;
    let mut checked = 0;
    for lib in version.libraries.iter() {
        let Some(path) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()).and_then(|a| a.path.as_ref()) else {
            continue;
        };
        // 和启动时同样的过滤：别的平台的 natives 本来就不该在 classpath 里
        if !minecraft::platform::is_usable(path) {
            continue;
        }
        checked += 1;
        let file = paths.libraries().join(path);
        if !Path::new(&file).is_file() {
            missing += 1;
            println!("缺库: {}", file.display());
        }
    }

    // client jar：加载器版本在父版本目录里
    let own = paths.client_jar(&id);
    println!(
        "client jar: {} ({})",
        own.display(),
        if own.is_file() { "存在" } else { "缺失" }
    );
    println!("本平台库: {checked} 个，缺 {missing} 个");

    // 加载器版本自己没有 jar，jar 在父版本目录
    let parent_jar = version
        .inherits_from
        .as_ref()
        .map(|p| paths.client_jar(p))
        .filter(|p| p.is_file());
    println!(
        "父版本 jar: {}",
        parent_jar
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "无".into())
    );
    Ok(())
}
