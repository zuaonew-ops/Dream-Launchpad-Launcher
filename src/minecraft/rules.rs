//! 版本 JSON 里的 rules 求值（决定某个库/参数在当前平台是否生效）。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OsRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

/// 当前平台名（与版本 JSON 的 os.name 对齐）。
pub fn os_name() -> &'static str {
    match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "osx",
        _ => "linux",
    }
}

/// 当前架构名（与版本 JSON 的 os.arch 对齐）。
pub fn os_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "x86",
        "aarch64" => "arm64",
        other => other,
    }
}

/// rules 为空表示默认允许；否则按顺序匹配，最后匹配的规则决定结果。
pub fn allowed(rules: &[Rule]) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut result = false;
    for rule in rules {
        if matches(rule) {
            result = rule.action == "allow";
        }
    }
    result
}

fn matches(rule: &Rule) -> bool {
    if let Some(os) = &rule.os {
        if let Some(name) = &os.name {
            if name != os_name() {
                return false;
            }
        }
        if let Some(arch) = &os.arch {
            if arch != os_arch() {
                return false;
            }
        }
    }
    // 启动器不支持 demo / 自定义分辨率等特性，带 features 的规则一律视为不匹配
    if rule.features.as_ref().is_some_and(|f| !f.is_empty()) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(json: &str) -> Rule {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn empty_rules_allow() {
        assert!(allowed(&[]));
    }

    #[test]
    fn allow_then_disallow_other_os() {
        let rules = vec![
            rule(r#"{"action":"allow"}"#),
            rule(r#"{"action":"disallow","os":{"name":"osx"}}"#),
        ];
        assert!(allowed(&rules));
    }

    #[test]
    fn allow_only_windows() {
        let rules = vec![rule(r#"{"action":"allow","os":{"name":"windows"}}"#)];
        assert_eq!(allowed(&rules), cfg!(target_os = "windows"));
    }

    #[test]
    fn feature_rules_do_not_match() {
        let rules = vec![rule(
            r#"{"action":"allow","features":{"is_demo_user":true}}"#,
        )];
        assert!(!allowed(&rules));
    }
}
