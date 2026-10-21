//! 离线账号：昵称 + 由昵称派生的 UUID。
//!
//! 存 `<root>/dlp/accounts.json`，`current` 是当前使用的账号 id
//! （id 就是离线 UUID，所以同名账号天然去重）。

use crate::auth;
use crate::paths::Paths;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Account {
    pub id: String,
    pub name: String,
    /// "offline" 或 "msa"
    #[serde(default = "offline_kind")]
    pub kind: String,
    /// 正版账号才有：Minecraft access token（24 小时过期）
    #[serde(default)]
    pub access_token: Option<String>,
    /// 正版账号才有：refresh token（90 天）
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// access token 到期的毫秒时间戳
    #[serde(default)]
    pub expires_at: Option<u64>,
    /// 皮肤 / 头像地址（正版才有）
    #[serde(default)]
    pub skin: Option<String>,
}

fn offline_kind() -> String {
    "offline".to_string()
}

impl Account {
    pub fn is_msa(&self) -> bool {
        self.kind == "msa"
    }

    /// access token 是否还能用（留 60 秒余量）
    pub fn token_alive(&self) -> bool {
        match self.expires_at {
            Some(at) => at > now_ms() + 60_000,
            None => false,
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Accounts {
    pub current: Option<String>,
    pub list: Vec<Account>,
}

impl Accounts {
    pub fn file(paths: &Paths) -> PathBuf {
        paths.root.join("dlp").join("accounts.json")
    }

    pub fn load(paths: &Paths) -> Accounts {
        let Ok(text) = std::fs::read_to_string(Self::file(paths)) else {
            return Accounts::default();
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
        std::fs::write(&file, text).map_err(|e| format!("写入账号失败: {e}"))
    }

    /// 添加账号；昵称非法或重名返回错误。第一个账号自动设为当前账号。
    pub fn add(&mut self, name: &str) -> Result<Account, String> {
        let name = name.trim();
        if !auth::valid_username(name) {
            return Err("昵称需 1–16 位，且不含空格".into());
        }
        let id = auth::offline_uuid(name);
        if self.list.iter().any(|a| a.id == id) {
            return Err(format!("已经有叫 {name} 的账号了"));
        }
        let account = Account {
            id,
            name: name.to_string(),
            kind: offline_kind(),
            access_token: None,
            refresh_token: None,
            expires_at: None,
            skin: None,
        };
        self.list.push(account.clone());
        if self.current.is_none() {
            self.current = Some(account.id.clone());
        }
        Ok(account)
    }

    pub fn remove(&mut self, id: &str) {
        self.list.retain(|a| a.id != id);
        if self.current.as_deref() == Some(id) {
            self.current = self.list.first().map(|a| a.id.clone());
        }
    }

    /// 切换当前账号；id 不存在返回错误。
    pub fn set_current(&mut self, id: &str) -> Result<(), String> {
        if !self.list.iter().any(|a| a.id == id) {
            return Err("账号不存在".into());
        }
        self.current = Some(id.to_string());
        Ok(())
    }

    /// 写入（或更新）一个正版账号，并设为当前账号
    pub fn upsert_msa(
        &mut self,
        uuid: &str,
        name: &str,
        access_token: &str,
        refresh_token: &str,
        expires_at: u64,
        skin: &str,
    ) -> Account {
        let account = Account {
            id: uuid.to_string(),
            name: name.to_string(),
            kind: "msa".to_string(),
            access_token: Some(access_token.to_string()),
            refresh_token: Some(refresh_token.to_string()),
            expires_at: Some(expires_at),
            skin: if skin.is_empty() { None } else { Some(skin.to_string()) },
        };
        if let Some(existing) = self.list.iter_mut().find(|a| a.id == account.id) {
            *existing = account.clone();
        } else {
            self.list.push(account.clone());
        }
        self.current = Some(account.id.clone());
        account
    }

    /// 用刷新后的令牌更新账号
    pub fn update_tokens(
        &mut self,
        id: &str,
        access_token: &str,
        refresh_token: &str,
        expires_at: u64,
    ) {
        if let Some(account) = self.list.iter_mut().find(|a| a.id == id) {
            account.access_token = Some(access_token.to_string());
            account.refresh_token = Some(refresh_token.to_string());
            account.expires_at = Some(expires_at);
        }
    }

    pub fn current_account(&self) -> Option<&Account> {
        let id = self.current.as_deref()?;
        self.list.iter().find(|a| a.id == id)
    }

    /// 启动游戏时用的昵称；一个账号都没有时退回默认名，保证能进游戏。
    pub fn current_name(&self) -> String {
        self.current_account()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Steve".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> (Accounts, Paths) {
        let dir = std::env::temp_dir().join(format!("dlp-acc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (Accounts::default(), Paths::new(&dir))
    }

    #[test]
    fn add_sets_current_and_rejects_duplicates() {
        let (mut acc, paths) = fresh();
        let added = acc.add("Steve").unwrap();
        assert_eq!(acc.current.as_deref(), Some(added.id.as_str()));
        assert!(acc.add("Steve").is_err());
        assert!(acc.add("").is_err());
        acc.save(&paths).unwrap();
        let loaded = Accounts::load(&paths);
        assert_eq!(loaded.list.len(), 1);
        assert_eq!(loaded.current_name(), "Steve");
    }

    #[test]
    fn remove_falls_back_to_remaining() {
        let (mut acc, _paths) = fresh();
        acc.add("Steve").unwrap();
        let alex = acc.add("Alex").unwrap();
        let steve = acc.list[0].id.clone();
        acc.remove(&steve);
        assert_eq!(acc.current.as_deref(), Some(alex.id.as_str()));
        acc.remove(&alex.id);
        assert!(acc.current.is_none());
        assert_eq!(acc.current_name(), "Steve");
    }

    #[test]
    fn set_current_requires_existing() {
        let (mut acc, _paths) = fresh();
        acc.add("Steve").unwrap();
        assert!(acc.set_current("nope").is_err());
        let id = acc.list[0].id.clone();
        assert!(acc.set_current(&id).is_ok());
    }
}
