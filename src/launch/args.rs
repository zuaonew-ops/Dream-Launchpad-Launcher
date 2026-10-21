//! 启动参数里的变量替换与条件参数收集。

use crate::auth;
use crate::launch::LaunchOptions;
use crate::minecraft::{rules::allowed, VersionJson};
use crate::paths::Paths;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// 版本 JSON 里会用到的全部变量。
#[allow(clippy::too_many_arguments)]
pub fn build_vars(
    paths: &Paths,
    version: &VersionJson,
    options: &LaunchOptions,
    game_dir: &Path,
    natives: &Path,
    classpath: &str,
    separator: &str,
) -> HashMap<&'static str, String> {
    let mut vars = HashMap::new();
    vars.insert("auth_player_name", options.username.to_string());
    vars.insert("version_name", options.id.to_string());
    vars.insert("game_directory", game_dir.to_string_lossy().to_string());
    vars.insert("assets_root", paths.assets().to_string_lossy().to_string());
    vars.insert("assets_index_name", assets_index_name(version));
    vars.insert("auth_uuid", options.uuid.to_string());
    vars.insert("auth_access_token", options.access_token.to_string());
    vars.insert(
        "auth_session",
        format!("token:{}:{}", options.access_token, options.uuid),
    );
    vars.insert(
        "user_type",
        if options.access_token == "0" { "legacy" } else { "msa" }.to_string(),
    );
    vars.insert(
        "version_type",
        version.kind.clone().unwrap_or_else(|| "release".into()),
    );
    vars.insert("natives_directory", natives.to_string_lossy().to_string());
    vars.insert("launcher_name", "DLP".to_string());
    vars.insert("launcher_version", crate::VERSION.to_string());
    vars.insert("classpath", classpath.to_string());
    vars.insert("classpath_separator", separator.to_string());
    vars.insert(
        "library_directory",
        paths.libraries().to_string_lossy().to_string(),
    );
    vars.insert("clientid", String::new());
    vars.insert("auth_xuid", "0".to_string());
    vars.insert("user_properties", "{}".to_string());
    vars
}

fn assets_index_name(version: &VersionJson) -> String {
    version
        .asset_index
        .as_ref()
        .map(|a| a.id.clone())
        .or_else(|| version.assets.clone())
        .unwrap_or_else(|| "legacy".into())
}

/// 收集一条参数：字符串直接替换；对象按 rules 判断是否生效。
pub fn collect(value: &Value, vars: &HashMap<&str, String>, out: &mut Vec<String>) {
    match value {
        Value::String(text) => out.push(substitute(text, vars)),
        Value::Object(map) => {
            let rules: Vec<crate::minecraft::rules::Rule> = map
                .get("rules")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();
            if !allowed(&rules) {
                return;
            }
            match map.get("value") {
                Some(Value::String(text)) => out.push(substitute(text, vars)),
                Some(Value::Array(items)) => {
                    for item in items {
                        if let Some(text) = item.as_str() {
                            out.push(substitute(text, vars));
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

pub fn substitute(template: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("${{{key}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_known_vars() {
        let mut vars = HashMap::new();
        vars.insert("auth_player_name", "Steve".to_string());
        assert_eq!(
            substitute("--username ${auth_player_name}", &vars),
            "--username Steve"
        );
    }

    #[test]
    fn leaves_unknown_vars() {
        let vars = HashMap::new();
        assert_eq!(substitute("${unknown}", &vars), "${unknown}");
    }

    #[test]
    fn collects_array_values() {
        let vars = HashMap::new();
        let value: Value = serde_json::from_str(r#"{"value":["-a","-b"]}"#).unwrap();
        let mut out = Vec::new();
        collect(&value, &vars, &mut out);
        assert_eq!(out, vec!["-a", "-b"]);
    }
}
