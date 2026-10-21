//! 验证 Java 运行时下载：`cargo run --example runtime_probe -- 21`

use dlp_core::{net::Progress, paths::Paths, runtime};

#[tokio::main]
async fn main() -> Result<(), String> {
    let major: u32 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "21".into())
        .parse()
        .map_err(|e| format!("参数不对: {e}"))?;

    let root = std::env::var_os("DLP_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(Paths::default_root);
    let paths = Paths::new(root);
    let client = reqwest::Client::builder()
        .user_agent("DLP-runtime-probe/0.1")
        .build()
        .map_err(|e| e.to_string())?;

    let progress = Progress::new(|stage, done, total| println!("  [{stage}] {done}/{total}"));
    let path = runtime::install(&client, &paths, major, Some(progress)).await?;
    println!("✅ Java {major}: {}", path.display());
    Ok(())
}
