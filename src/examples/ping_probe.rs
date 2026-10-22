//! 开发用探针：查服务器状态。
//!
//! 用法：`cargo run -p dlp-core --example ping_probe -- <host>[:port]`

use dlp_core::content;

#[tokio::main]
async fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| "mc.hypixel.net".into());
    let (host, port) = match arg.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().unwrap_or(25565)),
        None => (arg.clone(), 25565),
    };
    let status = content::ping(&host, port).await;
    println!(
        "{}:{} → {} | {} | {}/{} | {}ms | {}",
        status.host,
        status.port,
        if status.online { "在线" } else { "离线" },
        status.motd,
        status.players,
        status.max_players,
        status.latency,
        status.error
    );
}
