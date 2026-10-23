//! 开发用探针：看一个 zip 被认成什么整合包格式。
//!
//! 用法：`cargo run -p dlp-core --example packfmt_probe -- <a.zip> [b.zip ...]`

use dlp_core::pack;

fn main() {
    for path in std::env::args().skip(1) {
        let file = std::path::Path::new(&path);
        match pack::install::preview(file) {
            Ok(p) => println!(
                "{} → 格式 {} | {} {} | MC {} | 加载器 {:?} | 清单文件 {} | overrides {}",
                file.file_name().unwrap_or_default().to_string_lossy(),
                p.format,
                p.name,
                p.version,
                if p.mc.is_empty() { "?" } else { &p.mc },
                p.loader,
                p.files,
                p.overrides
            ),
            Err(e) => println!("{} → 失败: {e}", file.display()),
        }
    }
}
