//! Minecraft 数据模型：版本清单、版本 JSON、规则求值。

pub mod manifest;
pub mod platform;
pub mod rules;
pub mod scan;
pub mod version;

pub use manifest::{fetch, VersionEntry, VersionManifest};
pub use rules::allowed;
pub use scan::LocalVersion;
pub use version::{load as load_version, VersionJson};

/// 愚人节版本：Mojang 清单里它们仍标成 release / snapshot，只能认 id。
///
/// 名单来自官网每年的愚人节快照（2026-10 核对）。新增年份时往这里加一行。
pub fn is_april_fools(id: &str) -> bool {
    const KNOWN: &[&str] = &[
        "15w14a",
        "2.0",
        "3D Shareware v1.34",
        "20w14infinite",
        "22w13oneblockatatime",
        "23w13a_or_b",
        "24w14potato",
        "25w14craftmine",
    ];
    if KNOWN.contains(&id) {
        return true;
    }
    let lower = id.to_ascii_lowercase();
    lower.contains("craftmine")
        || lower.contains("oneblockatatime")
        || lower.contains("a_or_b")
        || lower.contains("24w14potato")
}

#[cfg(test)]
mod april_tests {
    use super::is_april_fools;

    #[test]
    fn recognises_april_fools_versions() {
        assert!(is_april_fools("25w14craftmine"));
        assert!(is_april_fools("24w14potato"));
        assert!(is_april_fools("3D Shareware v1.34"));
        assert!(!is_april_fools("26.3"));
        assert!(!is_april_fools("1.20.1"));
    }
}
