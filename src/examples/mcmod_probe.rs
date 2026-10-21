//! 验证 MC 百科查询：`cargo run --example mcmod_probe -- ModernFix`
//!
//! 逻辑与 `mcmod_url` 命令一致：搜一次，取第一个 class/NNNN.html。

#[tokio::main]
async fn main() -> Result<(), String> {
    let title = std::env::args().nth(1).unwrap_or_else(|| "ModernFix".into());
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .build()
        .map_err(|e| e.to_string())?;

    let body = client
        .get("https://search.mcmod.cn/s")
        .query(&[("key", title.as_str())])
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    println!("HTML 长度 {}", body.len());
    match first_mcmod_class(&body) {
        Some(url) => println!("项目页: {url}"),
        None => println!("没找到项目页（会退回搜索页）"),
    }
    Ok(())
}

fn first_mcmod_class(html: &str) -> Option<String> {
    const PREFIX: &str = "https://www.mcmod.cn/class/";
    let start = html.find(PREFIX)? + PREFIX.len();
    let rest = &html[start..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() || !rest[digits.len()..].starts_with(".html") {
        return None;
    }
    Some(format!("{PREFIX}{digits}.html"))
}
