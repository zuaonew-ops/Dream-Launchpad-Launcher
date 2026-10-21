//! 开发用探针：构建并拉起一次游戏。
//!
//! 用法：`cargo run -p dlp-core --example launch_probe -- 1.20.1 Steve`

use dlp_core::{java, launch, minecraft, paths::Paths};

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let id = args.next().unwrap_or_else(|| "1.20.1".to_string());
    let username = args.next().unwrap_or_else(|| "Steve".to_string());

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
            game_args: &[],
        },
    )?;
    println!("主类/参数数量: {}", plan.args.len());
    println!("前 6 个参数: {:?}", &plan.args[..6.min(plan.args.len())]);

    let pid = launch::spawn(&plan, &paths)?.id();
    println!("已启动，pid = {pid}");
    Ok(())
}
