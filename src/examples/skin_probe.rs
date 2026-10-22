//! 开发用探针：把一张 PNG 导入皮肤库并装给某个玩家。
//!
//! 用法：`cargo run -p dlp-core --example skin_probe -- <png 路径> [玩家名]`

use dlp_core::{content, paths::Paths};

fn main() -> Result<(), String> {
    let src = std::env::args()
        .nth(1)
        .ok_or_else(|| "用法: skin_probe <png 路径> [玩家名]".to_string())?;
    let player = std::env::args().nth(2).unwrap_or_else(|| "Steve".into());

    let paths = Paths::new(Paths::default_root());
    let dir = paths.root.clone();
    println!("游戏目录: {}", dir.display());

    let name = content::import_skin(&dir, std::path::Path::new(&src), "我的皮肤")?;
    println!("导入: {name}");
    for skin in content::list_skins(&dir) {
        println!("  库里有 {} {}×{} {} 字节", skin.name, skin.width, skin.height, skin.bytes);
    }

    println!("装 CustomSkinLoader: {}", content::has_customskinloader(&dir));
    let placed = content::apply_skin(&dir, &name, &player)?;
    println!("装给 {player}: {placed}");
    Ok(())
}
