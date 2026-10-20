//! 开发用探针：在终端里跑一遍安装流程（不经过界面）。
//!
//! 用法：`cargo run -p dlp-core --example install_probe -- 1.20.1`

use dlp_core::{install, minecraft, net::Progress, paths::Paths};

#[tokio::main]
async fn main() -> Result<(), String> {
    let id = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1.20.1".to_string());
    let paths = Paths::new(Paths::default_root());
    println!("数据目录: {}", paths.root.display());

    let client = reqwest::Client::builder()
        .user_agent("DLP-probe")
        .build()
        .map_err(|e| e.to_string())?;

    let manifest = minecraft::fetch(&client).await?;
    let entry = manifest
        .versions
        .into_iter()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("清单里没有 {id}"))?;
    println!("版本: {} ({})", entry.id, entry.kind);

    install::ensure_version_json(&client, &paths, &entry).await?;

    let last = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let progress = Progress::new(move |stage, done, total| {
        let mut previous = last.lock().unwrap();
        if *previous != stage {
            println!("\n[{stage}]");
            *previous = stage.to_string();
        }
        if done == total || done % 128 == 0 {
            println!("  {done}/{total}");
        }
    });

    install::install(&client, &paths, &id, progress).await?;
    println!("\n安装完成: {id} -> {}", paths.version_dir(&id).display());
    Ok(())
}
