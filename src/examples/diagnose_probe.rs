//! 开发用探针：往数据目录写一段日志，看看诊断给出什么。
//!
//! 用法：`cargo run -p dlp-core --example diagnose_probe -- <日志内容>`

use dlp_core::content::diagnose;
use dlp_core::paths::Paths;

fn main() {
    let text = std::env::args().nth(1).unwrap_or_else(|| {
        "[19:00:00] [main/ERROR]: java.lang.OutOfMemoryError: Java heap space".into()
    });
    let paths = Paths::new(Paths::default_root());
    let logs = paths.root.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("latest.log"), &text).unwrap();

    match diagnose::after_exit(&paths, Some(1)) {
        Some(d) => {
            println!("标题: {}", d.title);
            for line in &d.advice {
                println!("  建议: {line}");
            }
            for line in &d.evidence {
                println!("  证据: {line}");
            }
        }
        None => println!("正常退出，无话可说"),
    }
}
