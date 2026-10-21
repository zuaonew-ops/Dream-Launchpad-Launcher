//! 每个版本独立的启动设置。
//!
//! 存 `<root>/dlp/version-settings.json`：
//! - key 为版本 id，`"*"` 是全局默认
//! - 读取时全局打底、版本覆盖，所以只改某一项也能生效

use crate::paths::Paths;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const GLOBAL_KEY: &str = "*";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct VersionSettings {
    /// 内存上限（MB）
    pub memory_mb: Option<u32>,
    /// 指定 Java 可执行文件；None = 按版本要求自动挑
    pub java_path: Option<String>,
    /// 版本隔离；None = 用全局默认
    pub isolate: Option<bool>,
    /// 额外 JVM 参数
    pub extra_jvm_args: Vec<String>,
}

impl VersionSettings {
    /// 用 `other` 覆盖自己（只覆盖有值的字段）
    pub fn overridden_by(&self, other: &VersionSettings) -> VersionSettings {
        VersionSettings {
            memory_mb: other.memory_mb.or(self.memory_mb),
            java_path: other.java_path.clone().or_else(|| self.java_path.clone()),
            isolate: other.isolate.or(self.isolate),
            extra_jvm_args: if other.extra_jvm_args.is_empty() {
                self.extra_jvm_args.clone()
            } else {
                other.extra_jvm_args.clone()
            },
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Store {
    versions: BTreeMap<String, VersionSettings>,
}

impl Store {
    pub fn file(paths: &Paths) -> PathBuf {
        paths.root.join("dlp").join("version-settings.json")
    }

    pub fn load(paths: &Paths) -> Store {
        let Ok(text) = std::fs::read_to_string(Self::file(paths)) else {
            return Store::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self, paths: &Paths) -> Result<(), String> {
        let file = Self::file(paths);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录 {} 失败: {e}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&file, text).map_err(|e| format!("写入设置失败: {e}"))
    }

    pub fn raw(&self, id: &str) -> VersionSettings {
        self.versions.get(id).cloned().unwrap_or_default()
    }

    /// 合并后的设置：全局默认 + 该版本覆盖
    pub fn merged(&self, id: &str) -> VersionSettings {
        let global = self.raw(GLOBAL_KEY);
        let own = self.raw(id);
        global.overridden_by(&own)
    }

    pub fn set(&mut self, id: &str, settings: VersionSettings) {
        self.versions.insert(id.to_string(), settings);
    }

    pub fn all(&self) -> &BTreeMap<String, VersionSettings> {
        &self.versions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_settings_override_global() {
        let mut store = Store::default();
        store.set(
            GLOBAL_KEY,
            VersionSettings {
                memory_mb: Some(2048),
                isolate: Some(true),
                ..Default::default()
            },
        );
        store.set(
            "1.20.1",
            VersionSettings {
                memory_mb: Some(4096),
                ..Default::default()
            },
        );

        let merged = store.merged("1.20.1");
        assert_eq!(merged.memory_mb, Some(4096), "版本自己设的 4G 要盖掉全局 2G");
        assert_eq!(merged.isolate, Some(true), "没覆盖的字段用全局默认");
    }

    #[test]
    fn settings_roundtrip_through_disk() {
        let dir = std::env::temp_dir().join(format!("dlp-settings-{}", std::process::id()));
        let paths = Paths::new(&dir);
        let mut store = Store::load(&paths);
        store.set(
            "26.3",
            VersionSettings {
                memory_mb: Some(6144),
                extra_jvm_args: vec!["-Dfoo=bar".into()],
                ..Default::default()
            },
        );
        store.save(&paths).unwrap();

        let reloaded = Store::load(&paths);
        let merged = reloaded.merged("26.3");
        assert_eq!(merged.memory_mb, Some(6144));
        assert_eq!(merged.extra_jvm_args, vec!["-Dfoo=bar".to_string()]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// 启动器级设置（不属于某个版本）：默认启动哪个版本。
///
/// 存 `<root>/dlp/app.json`。没设置过就返回空，前端退回「已安装的第一个版本」。
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct AppSettings {
    pub default_version: Option<String>,
}

impl AppSettings {
    pub fn file(paths: &Paths) -> PathBuf {
        paths.root.join("dlp").join("app.json")
    }

    pub fn load(paths: &Paths) -> AppSettings {
        let Ok(text) = std::fs::read_to_string(Self::file(paths)) else {
            return AppSettings::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self, paths: &Paths) -> Result<(), String> {
        let file = Self::file(paths);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录 {} 失败: {e}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&file, text).map_err(|e| format!("写入设置失败: {e}"))
    }
}
