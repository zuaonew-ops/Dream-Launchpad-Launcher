//! 版本 JSON 的解析、继承链合并与平台筛选。

use crate::paths::Paths;
use super::rules::{allowed, os_arch, os_name, Rule};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default)]
    pub inherits_from: Option<String>,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub main_class: Option<String>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub asset_index: Option<AssetIndex>,
    #[serde(default)]
    pub downloads: Option<Downloads>,
    #[serde(default)]
    pub java_version: Option<JavaVersion>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub logging: Option<Logging>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub id: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub total_size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Downloads {
    #[serde(default)]
    pub client: Option<Artifact>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: u32,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub downloads: Option<LibDownloads>,
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub extract: Option<Extract>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LibDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: HashMap<String, Artifact>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Artifact {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Value>,
    #[serde(default)]
    pub jvm: Vec<Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Logging {
    #[serde(default)]
    pub client: Option<LoggingClient>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoggingClient {
    #[serde(default)]
    pub argument: Option<String>,
    #[serde(default)]
    pub file: Option<Artifact>,
}

impl VersionJson {
    /// 当前平台生效的库（父版本已合并进来）。
    pub fn active_libraries(&self) -> impl Iterator<Item = &Library> {
        self.libraries.iter().filter(|l| allowed(&l.rules))
    }

    /// natives 分类器键，例如 `natives-windows-64`。
    pub fn native_key(&self, lib: &Library) -> Option<String> {
        let natives = lib.natives.as_ref()?;
        let raw = natives.get(os_name())?;
        Some(raw.replace("${arch}", if os_arch() == "x86" { "32" } else { "64" }))
    }
}

/// 读取并合并继承链（`inheritsFrom`）。
pub fn load(paths: &Paths, id: &str) -> Result<VersionJson, String> {
    let path = paths.version_json(id);
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    let child: Value = serde_json::from_str(&raw).map_err(|e| format!("解析 {id}.json 失败: {e}"))?;

    let merged = match child.get("inheritsFrom").and_then(Value::as_str) {
        Some(parent_id) => {
            let parent = load(paths, parent_id)?;
            let parent_value =
                serde_json::to_value(&parent).map_err(|e| format!("序列化父版本失败: {e}"))?;
            merge(parent_value, child)
        }
        None => child,
    };

    serde_json::from_value(merged).map_err(|e| format!("合并后的 {id}.json 不合法: {e}"))
}

/// 父版本在前、子版本覆盖；`libraries` 与 `arguments` 按数组拼接。
pub fn merge(parent: Value, child: Value) -> Value {
    let Value::Object(child_map) = child else {
        return parent;
    };
    let Value::Object(mut out) = parent else {
        return Value::Object(child_map);
    };
    for (key, value) in child_map {
        match key.as_str() {
            "inheritsFrom" => {}
            "libraries" => {
                let mut list = out
                    .remove("libraries")
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default();
                if let Some(extra) = value.as_array() {
                    list.extend(extra.iter().cloned());
                }
                out.insert(key, Value::Array(list));
            }
            "arguments" => {
                let mut merged = out
                    .remove("arguments")
                    .unwrap_or_else(|| Value::Object(Default::default()));
                if let (Some(dst), Some(src)) = (merged.as_object_mut(), value.as_object()) {
                    for (arg_key, arg_value) in src {
                        let mut list = dst
                            .remove(arg_key)
                            .and_then(|v| v.as_array().cloned())
                            .unwrap_or_default();
                        if let Some(extra) = arg_value.as_array() {
                            list.extend(extra.iter().cloned());
                        }
                        dst.insert(arg_key.clone(), Value::Array(list));
                    }
                }
                out.insert(key, merged);
            }
            _ => {
                out.insert(key, value);
            }
        }
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_appends_libraries_and_keeps_child_scalars() {
        let parent: Value = serde_json::from_str(
            r#"{"id":"p","libraries":[{"name":"a"}],"mainClass":"P","arguments":{"game":["g1"],"jvm":["j1"]}}"#,
        )
        .unwrap();
        let child: Value = serde_json::from_str(
            r#"{"id":"c","libraries":[{"name":"b"}],"mainClass":"C","arguments":{"game":["g2"]}}"#,
        )
        .unwrap();
        let merged = merge(parent, child);
        assert_eq!(merged["mainClass"], "C");
        assert_eq!(merged["libraries"].as_array().unwrap().len(), 2);
        assert_eq!(merged["arguments"]["game"].as_array().unwrap().len(), 2);
        assert_eq!(merged["arguments"]["jvm"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn native_key_replaces_arch() {
        let lib: Library = serde_json::from_str(
            r#"{"name":"org.lwjgl:lwjgl:3","natives":{"windows":"natives-windows-${arch}"}}"#,
        )
        .unwrap();
        let v = VersionJson::default();
        let key = v.native_key(&lib).unwrap();
        assert!(key.starts_with("natives-windows-"));
        assert!(!key.contains("${arch}"));
    }
}
