//! 扫描本地 `versions/` 目录：不管是谁装的，只要在磁盘上就算数。

use crate::paths::Paths;
use serde::Serialize;
use std::fs;

#[derive(Serialize, Clone, Debug)]
pub struct LocalVersion {
    /// DLP 装的版本（目录里有 .dlp-installed 标记）
    pub dlp_installed: bool,
    pub id: String,
    /// 版本类型：release / snapshot / old_beta / custom
    pub kind: String,
    pub release_time: String,
    pub has_json: bool,
    pub has_jar: bool,
    pub inherits_from: Option<String>,
}

/// 列出 `versions/` 下所有可识别的版本（按时间倒序）。
pub fn scan(paths: &Paths) -> Vec<LocalVersion> {
    let dir = paths.versions();
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else {
        return out;
    };

    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        let json_path = entry.path().join(format!("{id}.json"));
        let jar_path = entry.path().join(format!("{id}.jar"));
        let has_json = json_path.is_file();
        let has_jar = jar_path.is_file();
        // 既没有 json 也没有 jar 的空目录不算版本
        if !has_json && !has_jar {
            continue;
        }

        let (kind, release_time, inherits_from) = if has_json {
            read_meta(&json_path).unwrap_or_else(|| ("custom".into(), String::new(), None))
        } else {
            ("custom".into(), String::new(), None)
        };

        out.push(LocalVersion {
            dlp_installed: crate::install::is_dlp_installed(paths, &id),
            id,
            kind,
            release_time,
            has_json,
            has_jar,
            inherits_from,
        });
    }

    out.sort_by(|a, b| b.release_time.cmp(&a.release_time).then_with(|| a.id.cmp(&b.id)));
    out
}

/// 只留「启动器能启动」的版本：必须有 json（jar 可以从父版本继承）。
pub fn launchable(paths: &Paths) -> Vec<LocalVersion> {
    scan(paths).into_iter().filter(|v| v.has_json).collect()
}

fn read_meta(path: &std::path::Path) -> Option<(String, String, Option<String>)> {
    let text = fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some((
        value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("custom")
            .to_string(),
        value
            .get("releaseTime")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        value
            .get("inheritsFrom")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_finds_json_versions_and_skips_empty_dirs() {
        let dir = crate::testdir::temp_dir("dlp-scan");
        let _ = fs::remove_dir_all(&dir);
        let version_dir = dir.join("versions").join("1.20.1");
        fs::create_dir_all(&version_dir).unwrap();
        fs::write(
            version_dir.join("1.20.1.json"),
            r#"{"id":"1.20.1","type":"release","releaseTime":"2023-06-12T12:00:00+00:00"}"#,
        )
        .unwrap();
        fs::create_dir_all(dir.join("versions").join("empty")).unwrap();

        let found = scan(&Paths::new(&dir));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "1.20.1");
        assert_eq!(found[0].kind, "release");
        assert!(found[0].has_json);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_versions_dir_is_empty() {
        let dir = crate::testdir::temp_dir("dlp-scan-none");
        assert!(scan(&Paths::new(&dir)).is_empty());
    }
}
