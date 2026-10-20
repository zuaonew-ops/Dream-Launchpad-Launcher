//! 启动器数据目录布局。
//!
//! 所有文件都放在一个根目录下（默认 `%APPDATA%\DreamLaunchpad`，Linux 用
//! `$XDG_DATA_HOME/DreamLaunchpad`），可用环境变量 `DLP_HOME` 覆盖。

use std::path::{Path, PathBuf};

pub const DIR_NAME: &str = "DreamLaunchpad";

#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 默认根目录。
    pub fn default_root() -> PathBuf {
        if let Some(custom) = std::env::var_os("DLP_HOME") {
            return PathBuf::from(custom);
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Path::new(&appdata).join(DIR_NAME);
        }
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            return Path::new(&xdg).join(DIR_NAME);
        }
        home_dir().join(".local").join("share").join(DIR_NAME)
    }

    pub fn versions(&self) -> PathBuf {
        self.root.join("versions")
    }

    pub fn version_dir(&self, id: &str) -> PathBuf {
        self.versions().join(id)
    }

    pub fn version_json(&self, id: &str) -> PathBuf {
        self.version_dir(id).join(format!("{id}.json"))
    }

    pub fn client_jar(&self, id: &str) -> PathBuf {
        self.version_dir(id).join(format!("{id}.jar"))
    }

    pub fn natives_dir(&self, id: &str) -> PathBuf {
        self.version_dir(id).join("natives")
    }

    pub fn libraries(&self) -> PathBuf {
        self.root.join("libraries")
    }

    pub fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn indexes(&self) -> PathBuf {
        self.assets().join("indexes")
    }

    pub fn objects(&self) -> PathBuf {
        self.assets().join("objects")
    }

    pub fn log_configs(&self) -> PathBuf {
        self.assets().join("log_configs")
    }

    pub fn is_installed(&self, id: &str) -> bool {
        self.version_json(id).is_file() && self.client_jar(id).is_file()
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_under_root() {
        let p = Paths::new("/data");
        assert!(p.version_json("1.20.1").ends_with("versions/1.20.1/1.20.1.json"));
        assert!(p.objects().ends_with("assets/objects"));
        assert!(p.libraries().ends_with("libraries"));
    }

    #[test]
    fn dlp_home_overrides_root() {
        // SAFETY: 单测线程内设置环境变量，不影响其它测试的断言内容
        std::env::set_var("DLP_HOME", "/tmp/dlp-test");
        assert_eq!(Paths::default_root(), PathBuf::from("/tmp/dlp-test"));
        std::env::remove_var("DLP_HOME");
    }
}
