//! 开发用探针：往游戏目录写/读 servers.dat。
//!
//! 用法：`cargo run -p dlp-core --example servers_probe -- ls|add <名字> <地址>|rm <地址>`

use dlp_core::{content, paths::Paths};

fn main() -> Result<(), String> {
    let paths = Paths::new(Paths::default_root());
    let dir = paths.root.clone();
    println!("游戏目录: {}", dir.display());

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("add") => {
            let name = args.get(1).cloned().unwrap_or_else(|| "测试服".into());
            let ip = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "play.example.com:25565".into());
            let list = content::add_server(&dir, &name, &ip)?;
            print_list(&list);
        }
        Some("rm") => {
            let ip = args.get(1).cloned().unwrap_or_default();
            let list = content::remove_server(&dir, &ip)?;
            print_list(&list);
        }
        _ => {
            let list = content::read_servers(&dir);
            print_list(&list);
        }
    }
    Ok(())
}

fn print_list(list: &[content::ServerEntry]) {
    println!("共 {} 台", list.len());
    for s in list {
        println!("  {} → {}{}", s.name, s.ip, if s.hidden { "（隐藏）" } else { "" });
    }
}
