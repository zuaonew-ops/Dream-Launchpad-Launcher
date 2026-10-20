//! 平台判定：区分「普通依赖库」和「natives 分类器 jar」。
//!
//! 现代版本（1.19+）把 natives 拆成了独立的库条目，命名形如
//! `lwjgl-3.3.1-natives-windows.jar`，且 rules 只按操作系统筛选、不区分架构，
//! 所以必须在下载和解压两处都按架构再过滤一次。

use super::rules::{os_arch, os_name};

/// 从路径里取出 `-natives-` 之后、`.jar` 之前的后缀。
pub fn native_suffix(path: &str) -> Option<&str> {
    let file = path.rsplit('/').next()?;
    let rest = file.strip_prefix("")?;
    let index = rest.find("-natives-")? + "-natives-".len();
    let tail = &rest[index..];
    tail.strip_suffix(".jar")
}

/// 是否为 natives 分类器 jar。
pub fn is_native_artifact(path: &str) -> bool {
    native_suffix(path).is_some()
}

/// 当前平台期望的 natives 后缀。
pub fn expected_suffix() -> &'static str {
    match (os_name(), os_arch()) {
        ("windows", "x86") => "windows-x86",
        ("windows", "arm64") => "windows-arm64",
        ("windows", _) => "windows",
        ("linux", "arm64") => "linux-arm64",
        ("linux", _) => "linux",
        ("osx", "arm64") => "macos-arm64",
        ("osx", _) => "macos",
        _ => "",
    }
}

/// 普通库恒为 true；natives 库只有后缀匹配当前平台才为 true。
pub fn is_usable(path: &str) -> bool {
    match native_suffix(path) {
        None => true,
        Some(suffix) => suffix == expected_suffix(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_natives() {
        assert!(is_native_artifact("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar"));
        assert!(!is_native_artifact("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar"));
    }

    #[test]
    fn picks_suffix() {
        assert_eq!(
            native_suffix("a/lwjgl-glfw-3.3.1-natives-windows-arm64.jar"),
            Some("windows-arm64")
        );
        assert_eq!(native_suffix("a/lwjgl-3.3.1.jar"), None);
    }

    #[test]
    fn filters_by_platform() {
        let win_x64 = "a/x-1.0-natives-windows.jar";
        let win_arm = "a/x-1.0-natives-windows-arm64.jar";
        let plain = "a/x-1.0.jar";
        assert!(is_usable(plain));
        assert_eq!(is_usable(win_x64), expected_suffix() == "windows");
        assert!(!is_usable(win_arm));
    }
}
