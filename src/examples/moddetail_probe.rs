//! 开发用探针：读模组详情。
//!
//! 用法：`cargo run -p dlp-core --example moddetail_probe -- <游戏目录> <jar 名>`

use dlp_core::content::modmeta;
use dlp_core::paths::Paths;
use std::path::PathBuf;

fn main() {
    let game = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| Paths::default_root());
    let file = std::env::args().nth(2).unwrap_or_default();
    match modmeta::detail(&game, &file) {
        Some(detail) => {
            println!("{} ({})", detail.name, detail.id);
            println!("  版本 {}", detail.version);
            println!("  加载器 {}", detail.loader);
            println!("  作者 {:?}", detail.authors);
            println!("  需要 MC {}", detail.mc_version);
            println!("  依赖 {:?}", detail.depends);
            println!("  描述 {}", detail.description.chars().take(160).collect::<String>());
        }
        None => println!("读不出详情: {file}"),
    }
}
