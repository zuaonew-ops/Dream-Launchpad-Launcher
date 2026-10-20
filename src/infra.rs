//! 路径与运行环境相关的基础设施。

use std::path::{Path, PathBuf};

/// 默认游戏目录名，与官方启动器保持一致。
pub const GAME_DIR_NAME: &str = ".minecraft";

/// 默认游戏目录：当前用户主目录下的 `.minecraft`。
pub fn default_game_dir() -> PathBuf {
    home_dir().join(GAME_DIR_NAME)
}

/// 某个版本的独立目录：`<root>/versions/<name>`。
pub fn instance_dir(root: &Path, name: &str) -> PathBuf {
    root.join("versions").join(name)
}

/// 取用户主目录；取不到时退回当前目录，避免启动阶段直接 panic。
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
    fn instance_dir_is_under_versions() {
        let dir = instance_dir(Path::new("/game"), "1.21.1");
        assert!(dir.ends_with(Path::new("versions").join("1.21.1")));
    }

    #[test]
    fn default_game_dir_ends_with_minecraft() {
        assert!(default_game_dir().ends_with(GAME_DIR_NAME));
    }
}
