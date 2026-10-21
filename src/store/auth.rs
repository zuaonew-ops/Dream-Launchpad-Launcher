//! 账号：离线 UUID（UUID v3，MD5 of `OfflinePlayer:<name>`）。

use md5::{Digest, Md5};

/// 离线模式的玩家 UUID（带连字符的标准格式）。
pub fn offline_uuid(name: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(format!("OfflinePlayer:{name}").as_bytes());
    let mut bytes = hasher.finalize();
    // UUID v3：把版本位设为 3，变体位设为 RFC 4122
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let h = hex::encode(bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// 名字是否可用作离线昵称（1–16 位，不含空白与控制字符）。
pub fn valid_username(name: &str) -> bool {
    let len = name.chars().count();
    (1..=16).contains(&len) && !name.chars().any(|c| c.is_whitespace() || c.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_matches_mojang_algorithm() {
        // 已知值：Notch -> b50ad385-829d-3141-a216-7e7d7539ba7f
        assert_eq!(
            offline_uuid("Notch"),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
    }

    #[test]
    fn username_validation() {
        assert!(valid_username("Steve"));
        assert!(!valid_username(""));
        assert!(!valid_username("有 空格"));
        assert!(!valid_username("12345678901234567"));
    }
}
