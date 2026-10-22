//! 开发用探针：构建并拉起一次游戏。
//!
//! 用法：`cargo run -p dlp-core --example launch_probe -- 1.20.1 Steve`

use dlp_core::{java, launch, minecraft, paths::Paths};

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let id = args.next().unwrap_or_else(|| "1.20.1".to_string());
    let username = args.next().unwrap_or_else(|| "Steve".to_string());
    // 第三个参数 = 直连地址，模拟启动器的「直连」按钮
    let quick_play: Vec<String> = match args.next() {
        Some(address) => vec!["--quickPlayMultiplayer".to_string(), address],
        None => vec![],
    };

    let paths = Paths::new(Paths::default_root());
    let version = minecraft::load_version(&paths, &id)?;
    let required = version
        .java_version
        .as_ref()
        .map(|j| j.major_version)
        .unwrap_or(8);

    let found = java::detect();
    for j in &found {
        println!("java {} @ {}", j.major, j.path);
    }
    let picked = java::pick(&found, required).ok_or_else(|| format!("缺少 Java {required}+"))?;
    println!("选用 Java {}: {}", picked.major, picked.path);

    let plan = launch::build(
        &paths,
        &version,
        &launch::LaunchOptions {
            id: &id,
            username: &username,
            uuid: &dlp_core::auth::offline_uuid(&username),
            access_token: "0",
            java: std::path::Path::new(&picked.path),
            memory_mb: 2048,
            isolate: true,
            extra_jvm_args: &[],
            game_args: &quick_play,
        },
    )?;
    println!("主类/参数数量: {}", plan.args.len());
    // 确认直连参数确实拼到了 main class 之后
    if let Some(pos) = plan.args.iter().position(|a| a == "--quickPlayMultiplayer") {
        println!("直连参数: {} {}", plan.args[pos], plan.args[pos + 1]);
    }
    println!("前 6 个参数: {:?}", &plan.args[..6.min(plan.args.len())]);
    // 打印 classpath：排查「找不到主类」就看这里有没有加载器的 jar
    if let Some(pos) = plan.args.iter().position(|a| a == "-cp" || a == "-classpath") {
        let cp = &plan.args[pos + 1];
        let parts: Vec<&str> = cp.split(';').collect();
        println!("classpath 条目 {} 个", parts.len());
        for part in parts.iter().filter(|p| p.contains("fabric") || p.contains("26.1") || p.contains("mixin")) {
            println!("  {part}");
        }
    } else {
        println!("命令行里没有 -cp！");
    }

    let pid = launch::spawn(&plan, &paths)?.id();
    println!("已启动，pid = {pid}");
    Ok(())
}
