//! 开发用探针：查市场能不能搜到整合包。
//!
//! 用法：`cargo run -p dlp-core --example market_probe -- <modrinth|curseforge> <kind> [关键词]`
//! CurseForge 需要 key：从 `DLP_HOME/dlp/app.json` 读。

use dlp_core::market::{Market, Source};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let source = match args.next().as_deref() {
        Some("curseforge") => Source::CurseForge,
        _ => Source::Modrinth,
    };
    let kind = args.next().unwrap_or_else(|| "modpack".into());
    let query = args.next().unwrap_or_default();

    let mut key = String::new();
    if let Some(home) = std::env::var_os("DLP_HOME") {
        let path = PathBuf::from(home).join("dlp").join("app.json");
        if let Ok(text) = std::fs::read_to_string(&path) {
            let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
            key = value
                .get("curseforge_api_key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
        }
    }
    println!("源 {:?} / 类型 {kind} / 关键词 {:?} / key 长度 {}", source, query, key.len());

    let market = Market::new(reqwest::Client::new(), key);
    match market
        .search(source, &query, &kind, None, None, Some("all"), 0, 3)
        .await
    {
        Ok(page) => {
            println!("共 {} 条", page.total);
            for hit in page.hits.iter().take(3) {
                println!("  {} | {} | {}", hit.title, hit.slug, hit.description.chars().take(40).collect::<String>());
            }
        }
        Err(e) => println!("失败: {e}"),
    }
}
