//! 开发用探针：预览并安装一个整合包（不经过界面）。
//!
//! 用法：`cargo run -p dlp-core --example pack_probe -- <zip 路径> [版本 id]`

use dlp_core::{net::Progress, pack, paths::Paths};

#[tokio::main]
async fn main() -> Result<(), String> {
    let zip = std::env::args()
        .nth(1)
        .ok_or_else(|| "用法: pack_probe <zip 路径> [版本 id]".to_string())?;
    let id = std::env::args().nth(2).unwrap_or_default();

    let paths = Paths::new(Paths::default_root());
    println!("数据目录: {}", paths.root.display());

    let preview = pack::install::preview(std::path::Path::new(&zip))?;
    println!(
        "格式={} 名称={} 版本={} MC={} 加载器={:?} 文件={} 大小={} 覆盖={}",
        preview.format,
        preview.name,
        preview.version,
        preview.mc,
        preview.loader,
        preview.files,
        preview.bytes,
        preview.overrides
    );
    for line in &preview.sample {
        println!("  · {line}");
    }

    let client = reqwest::Client::builder()
        .user_agent("DLP-probe")
        .build()
        .map_err(|e| e.to_string())?;
    let last = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let progress = Progress::new(move |stage, done, total, detail| {
        let mut previous = last.lock().unwrap();
        if *previous != stage {
            println!("\n[{stage}]");
            *previous = stage.to_string();
        }
        if done == total || done % 32 == 0 {
            println!("  {done}/{total} · {} · {}", detail.current(), detail.speed());
        }
    });

    let target = pack::install::install(&client, &paths, std::path::Path::new(&zip), &id, "", Some(progress)).await?;
    println!("\n装好了: {target}");
    println!("目录: {}", paths.version_dir(&target).display());
    Ok(())
}
