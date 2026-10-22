//! 复制 / 重命名版本目录。
//!
//! 版本目录里三样东西都带着版本 id：目录名、`<id>.json`、`<id>.jar`，
//! 复制或改名时必须一起改，否则按 id 找 jar 会落空。

use crate::paths::Paths;
use std::path::Path;

/// 把 `from` 复制成 `to`（`to` 已存在会被清掉）。
pub fn clone_version(paths: &Paths, from: &str, to: &str) -> Result<(), String> {
    if from == to {
        return Err("复制出来的名字不能和原来一样".into());
    }
    if !paths.version_json(from).is_file() {
        return Err(format!("版本 {from} 没有版本 JSON，复制不了"));
    }
    let src = paths.version_dir(from);
    let dst = paths.version_dir(to);
    if dst.exists() {
        std::fs::remove_dir_all(&dst).map_err(|e| format!("清理旧目录失败: {e}"))?;
    }
    copy_dir(&src, &dst)?;

    // JSON 里写着自己的 id，继承关系也可能指向它
    let json = dst.join(format!("{from}.json"));
    if json.is_file() {
        let text = std::fs::read_to_string(&json).map_err(|e| e.to_string())?;
        let mut value: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("解析版本 JSON 失败: {e}"))?;
        value["id"] = serde_json::Value::String(to.to_string());
        if value.get("inheritsFrom").and_then(|v| v.as_str()) == Some(from) {
            value["inheritsFrom"] = serde_json::Value::String(to.to_string());
        }
        let out = dst.join(format!("{to}.json"));
        std::fs::write(
            &out,
            serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("写入 {} 失败: {e}", out.display()))?;
        let _ = std::fs::remove_file(&json);
    }
    // jar 也跟着改名
    let jar = dst.join(format!("{from}.jar"));
    if jar.is_file() {
        let _ = std::fs::rename(&jar, dst.join(format!("{to}.jar")));
    }
    Ok(())
}

/// 改名为 `to`（先复制再删原目录）。
pub fn rename_version(paths: &Paths, from: &str, to: &str) -> Result<(), String> {
    clone_version(paths, from, to)?;
    let src = paths.version_dir(from);
    std::fs::remove_dir_all(&src).map_err(|e| format!("删除旧版本失败: {e}"))
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("创建 {} 失败: {e}", to.display()))?;
    let entries = std::fs::read_dir(from).map_err(|e| format!("读 {} 失败: {e}", from.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            // 没下完的临时文件不带过去
            if path.extension().map(|e| e == "part").unwrap_or(false) {
                continue;
            }
            std::fs::copy(&path, &target)
                .map_err(|e| format!("复制 {} 失败: {e}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_rewrites_id_and_jar() {
        let paths = Paths::new(crate::testdir::temp_dir("dlp-clone"));
        let from = paths.version_dir("a");
        std::fs::create_dir_all(&from).unwrap();
        std::fs::write(
            from.join("a.json"),
            r#"{"id":"a","inheritsFrom":"a"}"#,
        )
        .unwrap();
        std::fs::write(from.join("a.jar"), b"jar").unwrap();

        clone_version(&paths, "a", "b").unwrap();

        let dst = paths.version_dir("b");
        assert!(dst.join("b.json").is_file());
        assert!(dst.join("b.jar").is_file());
        assert!(!dst.join("a.json").exists());

        let text = std::fs::read_to_string(dst.join("b.json")).unwrap();
        assert!(text.contains("\"id\": \"b\""));
        assert!(text.contains("\"inheritsFrom\": \"b\""));
    }

    #[test]
    fn rename_removes_the_old_one() {
        let paths = Paths::new(crate::testdir::temp_dir("dlp-rename"));
        let from = paths.version_dir("x");
        std::fs::create_dir_all(&from).unwrap();
        std::fs::write(from.join("x.json"), r#"{"id":"x"}"#).unwrap();

        rename_version(&paths, "x", "y").unwrap();
        assert!(!paths.version_dir("x").exists());
        assert!(paths.version_dir("y").join("y.json").is_file());
    }
}
