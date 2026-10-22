//! 读一个 servers.dat，逐条打印（验证游戏重写后的文件还能读回来）。
use dlp_core::content;
fn main() {
    let dir = std::env::args().nth(1).expect("用法: servers_probe2 <游戏目录>");
    let list = content::read_servers(std::path::Path::new(&dir));
    println!("{} 条", list.len());
    for server in list {
        println!("  {} | {}", server.name, server.ip);
    }
}
