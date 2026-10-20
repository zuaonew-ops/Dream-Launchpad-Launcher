//! 启动游戏：构建参数并拉起 Java 进程。

pub mod args;

use crate::auth;
use crate::install::log_config_file;
use crate::minecraft::VersionJson;
use crate::paths::Paths;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// 游戏内默认语言（首次启动时写入 options.txt）。
pub const DEFAULT_LANGUAGE: &str = "zh_cn";

pub struct LaunchOptions<'a> {
    pub id: &'a str,
    pub username: &'a str,
    pub java: &'a Path,
    pub memory_mb: u32,
    /// 版本隔离：每个版本用自己的游戏目录（saves / mods 互不干扰）
    pub isolate: bool,
    /// 该版本额外的 JVM 参数
    pub extra_jvm_args: &'a [String],
}

#[derive(Debug, Clone)]
pub struct LaunchPlan {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub game_dir: PathBuf,
}

/// 构建启动计划（不真正启动）。
pub fn build(
    paths: &Paths,
    version: &VersionJson,
    options: &LaunchOptions,
) -> Result<LaunchPlan, String> {
    if !auth::valid_username(options.username) {
        return Err("昵称不合法：1–16 个字符，不能含空格".into());
    }
    let main_class = version
        .main_class
        .clone()
        .ok_or_else(|| "版本 JSON 缺少 mainClass".to_string())?;

    let game_dir = if options.isolate {
        paths.version_dir(options.id)
    } else {
        paths.root.clone()
    };
    std::fs::create_dir_all(&game_dir)
        .map_err(|e| format!("创建游戏目录 {} 失败: {e}", game_dir.display()))?;

    let separator = if cfg!(windows) { ";" } else { ":" };
    let classpath = classpath(paths, version, options.id).join(separator);

    let natives = paths.natives_dir(options.id);
    std::fs::create_dir_all(&natives).map_err(|e| format!("创建 natives 目录失败: {e}"))?;

    let vars = args::build_vars(
        paths,
        version,
        options,
        &game_dir,
        &natives,
        &classpath,
        separator,
    );
    let mut argv: Vec<String> = Vec::new();

    // JVM 参数：先按版本 JSON 里声明的顺序，再补内存 / natives / 日志配置
    if let Some(arguments) = &version.arguments {
        for value in &arguments.jvm {
            args::collect(value, &vars, &mut argv);
        }
    }
    for extra in options.extra_jvm_args {
        argv.push(extra.clone());
    }
    argv.push(format!("-Xmx{}M", options.memory_mb));
    argv.push(format!("-Djava.library.path={}", natives.display()));

    if let (Some(argument), Some(file)) = (
        version
            .logging
            .as_ref()
            .and_then(|l| l.client.as_ref())
            .and_then(|c| c.argument.clone()),
        log_config_file(version),
    ) {
        let name = file
            .path
            .clone()
            .unwrap_or_else(|| "client-log.xml".to_string());
        let value = args::substitute(
            &argument.replace(
                "${path}",
                &paths.log_configs().join(name).to_string_lossy(),
            ),
            &vars,
        );
        argv.push(value);
    }

    // 老版本（1.13 以前）没有 arguments.jvm，需要手动给 -cp
    if version.arguments.is_none() {
        argv.push("-cp".into());
        argv.push(classpath.clone());
    }

    argv.push(main_class);

    if let Some(arguments) = &version.arguments {
        for value in &arguments.game {
            args::collect(value, &vars, &mut argv);
        }
    } else if let Some(legacy) = &version.minecraft_arguments {
        for token in legacy.split_whitespace() {
            argv.push(args::substitute(token, &vars));
        }
    }

    Ok(LaunchPlan {
        program: options.java.to_path_buf(),
        args: argv,
        cwd: game_dir.clone(),
        game_dir,
    })
}

/// 依赖库 + 客户端 jar 组成的 classpath。
fn classpath(paths: &Paths, version: &VersionJson, id: &str) -> Vec<String> {
    let mut entries: Vec<String> = version
        .active_libraries()
        .filter_map(|lib| lib.downloads.as_ref()?.artifact.as_ref()?.path.as_ref())
        .map(|p| {
            paths
                .libraries()
                .join(p)
                .to_string_lossy()
                .to_string()
        })
        .collect();
    entries.push(paths.client_jar(id).to_string_lossy().to_string());
    entries
}

/// 首次启动时把游戏语言设成中文（只在 options.txt 不存在时写，不覆盖玩家设置）。
pub fn ensure_default_language(game_dir: &Path) -> Result<(), String> {
    let options = game_dir.join("options.txt");
    if options.is_file() {
        return Ok(());
    }
    std::fs::write(&options, format!("lang:{DEFAULT_LANGUAGE}\n"))
        .map_err(|e| format!("写入 options.txt 失败: {e}"))
}

/// 真正拉起游戏进程，返回 pid。输出写入 `<root>/logs/<版本>.log`。
pub fn spawn(plan: &LaunchPlan, paths: &Paths) -> Result<std::process::Child, String> {
    ensure_default_language(&plan.game_dir)?;

    let logs = paths.root.join("logs");
    std::fs::create_dir_all(&logs).map_err(|e| format!("创建日志目录失败: {e}"))?;
    let name = plan
        .game_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "game".into());
    let log_path = logs.join(format!("{name}.log"));
    let stdout = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("打开日志文件失败: {e}"))?;
    let stderr = stdout
        .try_clone()
        .map_err(|e| format!("复制日志句柄失败: {e}"))?;

    let mut command = Command::new(&plan.program);
    command
        .args(&plan.args)
        .current_dir(&plan.cwd)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    // CREATE_NO_WINDOW：GUI 进程拉 console 程序时别弹黑框
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let child = command
        .spawn()
        .map_err(|e| format!("启动 Java 失败: {e}"))?;

    Ok(child)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_default_language_only_once() {
        let dir = std::env::temp_dir().join("dlp-lang-test");
        std::fs::create_dir_all(&dir).unwrap();
        let options = dir.join("options.txt");
        let _ = std::fs::remove_file(&options);

        ensure_default_language(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(&options).unwrap(), "lang:zh_cn\n");

        std::fs::write(&options, "lang:en_us\n").unwrap();
        ensure_default_language(&dir).unwrap();
        assert_eq!(std::fs::read_to_string(&options).unwrap(), "lang:en_us\n");
    }
}
