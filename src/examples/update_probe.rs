//! 开发用探针：检查某个版本 mods 目录里的模组有没有新版本。
//!
//! 用法：`cargo run -p dlp-core --example update_probe -- <版本 id> [mc] [loader]`

use dlp_core::{
    market::{identify, is_newer, latest},
    net::sha1_file,
    paths::Paths,
};

#[tokio::main]
async fn main() -> Result<(), String> {
    let id = std::env::args().nth(1).unwrap_or_else(|| "MODTEST".into());
    let mc = std::env::args().nth(2).unwrap_or_else(|| "1.20.1".into());
    let loader = std::env::args().nth(3).unwrap_or_else(|| "fabric".into());

    let paths = Paths::new(Paths::default_root());
    let dir = paths.version_dir(&id).join("mods");
    println!("mods 目录: {}", dir.display());

    let entries = std::fs::read_dir(&dir).map_err(|e| format!("读目录失败: {e}"))?;
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.to_ascii_lowercase().ends_with(".jar") {
            continue;
        }
        let sha1 = sha1_file(&entry.path()).map_err(|e| format!("算 SHA1 失败: {e}"))?;
        println!("  {name} → sha1 {}", &sha1[..12]);
        files.push((name, sha1));
    }

    let client = reqwest::Client::builder()
        .user_agent("DLP-probe")
        .build()
        .map_err(|e| e.to_string())?;

    let identities = identify(&client, &files).await?;
    println!("认出 {} 个（不在 Modrinth 上的认不出来）", identities.len());

    for item in identities {
        println!(
            "  {} = {} {}（当前 {}）",
            item.file_name, item.project_title, item.project_slug, item.version_number
        );
        match latest(&client, &item.project_id, &mc, &loader).await? {
            Some((version, url, filename)) => {
                let newer = is_newer(&version, &item.version_number);
                println!(
                    "    最新 {version} {} → {filename}",
                    if newer { "（可更新）" } else { "（已最新）" }
                );
                if newer {
                    println!("    {url}");
                }
            }
            None => println!("    这个版本 + 加载器下没有兼容文件"),
        }
    }
    Ok(())
}
