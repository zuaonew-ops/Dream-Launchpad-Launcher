//! 开发用探针：把一个版本导出成 .mrpack 再装回来（验证闭环）。
//!
//! 用法：`cargo run -p dlp-core --example export_probe -- <版本 id> <输出路径>`

use dlp_core::{pack, paths::Paths};

fn main() -> Result<(), String> {
    let id = std::env::args().nth(1).unwrap_or_else(|| "TestPack".into());
    let target = std::env::args()
        .nth(2)
        .unwrap_or_else(|| format!("D:/dlp-artifacts/{id}.mrpack"));

    let paths = Paths::new(Paths::default_root());
    println!("数据目录: {}", paths.root.display());

    let result = pack::export::export(&paths, &id, std::path::Path::new(&target))?;
    println!(
        "导出完成: {} （{} 个文件，{} 字节，MC {}，加载器 {:?}）",
        result.path, result.files, result.bytes, result.mc, result.loader
    );

    // 再读一遍，确认能被自己的解析器认出来
    let preview = pack::install::preview(std::path::Path::new(&target))?;
    println!(
        "回读: 名称={} MC={} 加载器={:?} 覆盖文件={}",
        preview.name, preview.mc, preview.loader, preview.overrides
    );
    Ok(())
}
