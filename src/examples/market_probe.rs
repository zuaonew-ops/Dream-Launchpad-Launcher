//! 验证 Modrinth 搜索 / 安装：
//! `cargo run --example market_probe -- search sodium mod 1.20.1 fabric`
//! `cargo run --example market_probe -- install sodium resourcepack 1.20.1`

use dlp_core::{market, net::Progress, paths::Paths, settings::Store};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        return Err("用法: market_probe <search|install> <项目> <种类> <mc版本> [加载器]".into());
    }

    let root = std::env::var_os("DLP_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(Paths::default_root);
    let paths = Paths::new(root);
    let client = reqwest::Client::builder()
        .user_agent("DLP-market-probe/0.1")
        .build()
        .map_err(|e| e.to_string())?;
    let progress = Progress::new(|stage, done, total| println!("  [{stage}] {done}/{total}"));

    let action = &args[0];
    let query = &args[1];
    let kind = &args[2];
    let mc = &args[3];
    let loader = args.get(4).map(String::as_str);

    match action.as_str() {
        "search" => {
            let page = market::search(&client, query, kind, Some(mc), loader, 0, 5).await?;
            println!("命中 {} 个（显示前 {}）", page.total, page.hits.len());
            for hit in &page.hits {
                println!("  {}  ({})  {}", hit.title, hit.slug, hit.downloads);
            }
        }
        "install" => {
            // 装到 DLP_HOME 下第一个匹配的版本目录
            let target = std::fs::read_dir(paths.versions())
                .map_err(|e| e.to_string())?
                .flatten()
                .find(|e| e.file_name().to_string_lossy().contains(mc))
                .ok_or_else(|| format!("没有找到包含 {mc} 的版本目录"))?;
            let settings = Store::load(&paths);
            let id = target.file_name().to_string_lossy().to_string();
            let game_dir = dlp_core::content::game_dir(
                &paths,
                &id,
                settings.merged(&id).isolate.unwrap_or(true),
            );
            println!("目标版本 {id} → {}", game_dir.display());
            let file = market::install(
                &client,
                &game_dir,
                query,
                kind,
                Some(mc),
                loader,
                Some(progress),
            )
            .await?;
            println!("✅ 已下载 {file}");
        }
        other => return Err(format!("未知动作: {other}")),
    }

    Ok(())
}
