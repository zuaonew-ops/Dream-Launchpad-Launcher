//! Java 运行时探测。

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct JavaInfo {
    pub path: String,
    pub version: String,
    pub major: u32,
}

/// 探测本机可用的 Java（去重，按版本降序）。
pub fn detect() -> Vec<JavaInfo> {
    let mut found: Vec<JavaInfo> = Vec::new();
    for path in candidates() {
        if found.iter().any(|j| j.path == path.to_string_lossy()) {
            continue;
        }
        if let Some(info) = probe(&path) {
            found.push(info);
        }
    }
    found.sort_by(|a, b| b.major.cmp(&a.major));
    found
}

fn java_binary(home: &Path) -> PathBuf {
    home.join("bin").join(if cfg!(windows) { "java.exe" } else { "java" })
}

fn candidates() -> Vec<PathBuf> {
    let mut list = Vec::new();

    if let Some(home) = std::env::var_os("JAVA_HOME") {
        list.push(java_binary(Path::new(&home)));
    }

    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let exe = dir.join(if cfg!(windows) { "java.exe" } else { "java" });
            if exe.is_file() {
                list.push(exe);
            }
        }
    }

    if cfg!(windows) {
        let bases = [
            r"C:\Program Files\Java",
            r"C:\Program Files\Eclipse Adoptium",
            r"C:\Program Files\Microsoft",
            r"C:\Program Files\Zulu",
            r"C:\Program Files\BellSoft",
            r"C:\Program Files\Amazon Corretto",
        ];
        for base in bases {
            push_subdirs(&mut list, Path::new(base));
        }
    } else {
        push_subdirs(&mut list, Path::new("/usr/lib/jvm"));
    }

    list.retain(|p| p.is_file());
    list
}

fn push_subdirs(list: &mut Vec<PathBuf>, base: &Path) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            list.push(java_binary(&path));
        }
    }
}

/// 跑一次 `java -version` 取版本号。
fn probe(exe: &Path) -> Option<JavaInfo> {
    let output = Command::new(exe).arg("-version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stderr);
    let first = text.lines().next().unwrap_or("");
    let version = extract_quoted(first)?;
    let major = parse_major(&version)?;
    Some(JavaInfo {
        path: exe.to_string_lossy().to_string(),
        version,
        major,
    })
}

fn extract_quoted(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let end = line[start..].find('"')? + start;
    Some(line[start..end].to_string())
}

fn parse_major(version: &str) -> Option<u32> {
    let head = version.split('.').next()?;
    let value: u32 = head.parse().ok()?;
    // "1.8.0_392" => 8
    if value == 1 {
        version.split('.').nth(1)?.parse().ok()
    } else {
        Some(value)
    }
}

/// 选出满足 `required` 的最合适 Java：优先等于要求的主版本，其次更大的最小者。
pub fn pick(list: &[JavaInfo], required: u32) -> Option<&JavaInfo> {
    list.iter()
        .filter(|j| j.major >= required)
        .min_by_key(|j| j.major)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_majors() {
        assert_eq!(parse_major("21.0.12"), Some(21));
        assert_eq!(parse_major("1.8.0_392"), Some(8));
        assert_eq!(parse_major("17"), Some(17));
    }

    #[test]
    fn picks_lowest_satisfying() {
        let list = vec![
            JavaInfo { path: "a".into(), version: "8".into(), major: 8 },
            JavaInfo { path: "b".into(), version: "21".into(), major: 21 },
            JavaInfo { path: "c".into(), version: "25".into(), major: 25 },
        ];
        assert_eq!(pick(&list, 17).unwrap().major, 21);
        assert!(pick(&list, 30).is_none());
    }
}
