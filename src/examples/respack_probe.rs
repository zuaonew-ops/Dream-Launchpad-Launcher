//! 开发用探针：读写资源包启用列表。
//!
//! 用法：`cargo run -p dlp-core --example respack_probe -- <游戏目录> [要启用的包名...]`

use dlp_core::content::options;
use std::path::PathBuf;

fn main() {
    let game = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .expect("用法: respack_probe <游戏目录> [包名...]");
    let packs: Vec<String> = std::env::args().skip(2).map(|n| format!("file/{n}")).collect();

    println!("当前启用: {:?}", options::read_resource_packs(&game));
    if !packs.is_empty() {
        options::write_resource_packs(&game, &packs).unwrap();
        println!("改成: {:?}", options::read_resource_packs(&game));
        println!("--- options.txt 里的 resourcePacks 行 ---");
        let text = std::fs::read_to_string(game.join("options.txt")).unwrap_or_default();
        for line in text.lines().filter(|l| l.starts_with("resourcePacks:")) {
            println!("{line}");
        }
    }
}
