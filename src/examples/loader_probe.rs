//! 开发用探针：查某个 MC 版本能装哪些加载器、每个有几个版本。

use dlp_core::loader::{self, LoaderKind};

#[tokio::main]
async fn main() {
    let mc = std::env::args().nth(1).unwrap_or_else(|| "1.12.2".into());
    let client = reqwest::Client::new();
    println!("MC {mc}");
    for kind in [
        LoaderKind::Fabric,
        LoaderKind::Quilt,
        LoaderKind::Forge,
        LoaderKind::NeoForge,
    ] {
        let result = match kind {
            LoaderKind::Forge | LoaderKind::NeoForge => {
                loader::installer_candidates(&client, &mc, kind).await
            }
            _ => loader::candidates(&client, &mc, kind).await,
        };
        match result {
            Ok(list) => {
                let head: Vec<String> = list.iter().take(3).map(|c| c.version.clone()).collect();
                println!("  {:?}  {} 个  {:?}", kind, list.len(), head);
            }
            Err(e) => println!("  {:?}  失败: {e}", kind),
        }
    }
}
