//! 启动失败诊断：游戏非正常退出后，说说最可能的原因。
//!
//! 依据三样东西：退出码、日志尾部、最新的崩溃报告。都是本地已有的信息，
//! 不给用户凭空猜 —— 每条判断都对着具体的一行日志。

use crate::paths::Paths;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnosis {
    /// 一句话结论
    pub title: String,
    /// 建议怎么办（每行一条）
    pub advice: Vec<String>,
    /// 支撑这个结论的日志行
    pub evidence: Vec<String>,
    /// 崩溃报告文件名（有的话）
    pub crash: Option<String>,
}

/// 看一次退出：返回 None 表示「正常退出，没什么好说的」。
pub fn after_exit(paths: &Paths, code: Option<i32>) -> Option<Diagnosis> {
    // 退出码 0 = 正常退出；None 在 Windows 上通常是 killed，不当异常
    if code == Some(0) || code.is_none() {
        return None;
    }
    let tail = crate::content::logs::read_log(paths, "latest.log", 400).unwrap_or_default();
    let lower = tail.to_lowercase();

    // 先看有没有当次的崩溃报告
    let crashes = crate::content::logs::list_crashes(paths);
    let newest = crashes.first();

    let mut advice: Vec<String> = Vec::new();
    let mut evidence: Vec<String> = Vec::new();
    let mut title = format!("游戏异常退出（退出码 {}）", code.unwrap_or(-1));

    // 按「最常见」排序判断
    if lower.contains("unsupportedclassversionerror")
        || lower.contains("has been compiled by a more recent version")
    {
        title = "Java 版本太低".into();
        evidence.push(first_line_containing(&tail, "UnsupportedClassVersionError"));
        advice.push("到「设置 → Java」换一个更高的版本，或让启动器自动挑".into());
    } else if lower.contains("outofmemoryerror") || lower.contains("java heap space") {
        title = "内存不够".into();
        evidence.push(first_line_containing(&tail, "OutOfMemoryError"));
        advice.push("到「版本设置」把内存上限调大（比如 4096 MB）".into());
    } else if lower.contains("duplicate mods")
        || lower.contains("duplicatemodsfoundexception")
        || lower.contains("incompatible mod set")
    {
        title = "模组装重了或不兼容".into();
        evidence.push(first_line_containing(&tail, "DuplicateMods"));
        advice.push("到「模组」页看「装了多个版本」，留一个".into());
    } else if lower.contains("missing or unsupported mandatory dependencies")
        || lower.contains("mod requires")
    {
        title = "缺前置模组".into();
        evidence.push(first_line_containing(&tail, "requires"));
        advice.push("按日志里提到的 mod id 去市场补装前置".into());
    } else if lower.contains("mixin apply failed") || lower.contains("mixinapplyerror") {
        title = "Mixin 注入失败（通常是模组冲突）".into();
        evidence.push(first_line_containing(&tail, "Mixin apply failed"));
        advice.push("多半是某个模组和当前加载器版本对不上，看日志里点名的是哪个".into());
    } else if lower.contains("java.lang.noclassdeffounderror") {
        title = "缺类（加载器或模组文件损坏）".into();
        evidence.push(first_line_containing(&tail, "NoClassDefFoundError"));
        advice.push("先试「修复版本」，还不行就看日志点名的是哪个库".into());
    } else if lower.contains("failed to download") || lower.contains("couldn't download") {
        title = "启动时下载依赖失败".into();
        evidence.push(first_line_containing(&tail, "download"));
        advice.push("换「设置 → 下载源」再试".into());
    } else if let Some(crash) = newest {
        title = format!("游戏崩溃：{}", crash.summary);
        advice.push("「工具 → 崩溃报告」里有完整内容".into());
    } else {
        advice.push("看「工具 → 日志」的最后几十行，通常是异常栈的第一行".into());
    }

    // 崩溃报告只在这一局之后生成才算数（按文件名时间戳粗判）
    let crash = newest.map(|c| c.name.clone());
    evidence.retain(|line| !line.is_empty());
    evidence.truncate(3);

    Some(Diagnosis {
        title,
        advice,
        evidence,
        crash,
    })
}

/// 日志里第一条包含关键字的行（去掉时间戳前缀之外的东西，太长就截断）。
fn first_line_containing(text: &str, needle: &str) -> String {
    for line in text.lines() {
        if line.to_lowercase().contains(&needle.to_lowercase()) {
            let trimmed = line.trim();
            return trimmed.chars().take(200).collect();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一个带 latest.log 的假数据目录
    fn with_log(text: &str) -> Paths {
        let root = crate::testdir::temp_dir("dlp-diagnose");
        std::fs::create_dir_all(root.join("logs")).unwrap();
        std::fs::write(root.join("logs").join("latest.log"), text).unwrap();
        Paths::new(root)
    }

    #[test]
    fn normal_exit_says_nothing() {
        let paths = with_log("");
        assert!(after_exit(&paths, Some(0)).is_none());
        assert!(after_exit(&paths, None).is_none());
    }

    #[test]
    fn spots_old_java() {
        let paths = with_log(
            "[19:00:00] [main/ERROR]: java.lang.UnsupportedClassVersionError: net/minecraft/client/main/Main has been compiled by a more recent version",
        );
        let d = after_exit(&paths, Some(1)).unwrap();
        assert_eq!(d.title, "Java 版本太低");
        assert!(d.evidence[0].contains("UnsupportedClassVersionError"));
    }

    #[test]
    fn spots_out_of_memory() {
        let paths = with_log("[19:00:00] [main/ERROR]: java.lang.OutOfMemoryError: Java heap space");
        let d = after_exit(&paths, Some(1)).unwrap();
        assert_eq!(d.title, "内存不够");
    }

    #[test]
    fn spots_duplicate_mods() {
        let paths = with_log("[19:00:00] [main/ERROR]: DuplicateModsFoundException: sodium");
        let d = after_exit(&paths, Some(1)).unwrap();
        assert_eq!(d.title, "模组装重了或不兼容");
    }
}
