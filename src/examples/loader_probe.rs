//! 手动验证加载器 / OptiFine 安装：
//! `cargo run --example loader_probe -- list fabric 1.20.1`
//! `cargo run --example loader_probe -- install fabric 1.20.1`
//! `cargo run --example loader_probe -- optifine 1.20.1`

use dlp_core::{java, loader, net::Progress, paths::Paths};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err(
            "用法: loader_probe <list|install> <fabric|quilt|forge|neoforge> <mc版本> [加载器版本] | loader_probe optifine <mc版本> [edition]".into(),
        );
    }

    let root = std::env::var_os("DLP_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(Paths::default_root);
    let paths = Paths::new(root);
    let client = reqwest::Client::builder()
        .user_agent("DLP-probe/0.1")
        .build()
        .map_err(|e| e.to_string())?;
    let progress = Progress::new(|stage, done, total| {
        println!("  [{stage}] {done}/{total}");
    });

    match args[0].as_str() {
        "list" => {
            let kind = loader::LoaderKind::parse(&args[1]).ok_or("未知加载器")?;
            let list = loader::candidates(&client, &args[2], kind).await?;
            println!("{kind:?} for {}: {} 个候选", args[2], list.len());
            for item in list.iter().take(5) {
                println!("  {} ({})", item.label, item.version);
            }
        }
        "install" => {
            let kind = loader::LoaderKind::parse(&args[1]).ok_or("未知加载器")?;
            let mc = &args[2];
            let version = match args.get(3) {
                Some(v) => v.clone(),
                None => {
                    let list = match kind {
                        loader::LoaderKind::Fabric | loader::LoaderKind::Quilt => {
                            loader::candidates(&client, mc, kind).await?
                        }
                        _ => loader::installer_candidates(&client, mc, kind).await?,
                    };
                    list.first()
                        .ok_or("没有可用版本")?
                        .version
                        .clone()
                }
            };
            println!("安装 {kind:?} {version} → MC {mc}");
            let id = match kind {
                loader::LoaderKind::Fabric | loader::LoaderKind::Quilt => {
                    loader::install_meta(&client, &paths, mc, kind, &version, Some(progress)).await?
                }
                _ => {
                    let found = java::detect();
                    let java_bin = java::pick(&found, 17).ok_or("没有 Java 17+")?;
                    let installer = loader::resolve_installer_path(&client, mc, kind, &version).await;
                    loader::install_with_installer(
                        &client,
                        &paths,
                        mc,
                        kind,
                        &version,
                        installer,
                        std::path::Path::new(&java_bin.path),
                        Some(progress),
                    )
                    .await?
                }
            };
            println!("✅ 生成版本: {id}（{}）", paths.version_dir(&id).display());
        }
        "optifine" => {
            let mc = &args[1];
            let list = loader::optifine_candidates(&client, mc).await?;
            println!("OptiFine for {mc}: {} 个", list.len());
            let edition = args
                .get(2)
                .cloned()
                .or_else(|| list.first().map(|c| c.version.clone()))
                .ok_or("没有 OptiFine 版本")?;
            println!("安装 OptiFine {edition}");
            let found = java::detect();
            let java_bin = java::pick(&found, 8).ok_or("没有 Java")?;
            let id = loader::install_optifine(
                &client,
                &paths,
                mc,
                &edition,
                std::path::Path::new(&java_bin.path),
                Some(progress),
            )
            .await?;
            println!("✅ 生成版本: {id}（{}）", paths.version_dir(&id).display());
        }
        other => return Err(format!("未知动作: {other}")),
    }

    Ok(())
}
