//! 微软正版登录（device code 流程）。
//!
//! 链式换票：MSA device code → Xbox Live → XSTS → Minecraft Services。
//! 只有最后一步的 access token 能用来启动游戏；它 24 小时过期，
//! 所以 refresh token（90 天）必须落盘，下次启动前静默刷新。
//!
//! 不需要 client secret —— 这属于微软定义的「公共客户端」，
//! 第三方启动器都是这么接的。client id 可由用户在设置里替换。

use serde::{Deserialize, Serialize};

/// 默认 client id：PrismLauncher 公开在源码里的公共客户端 id，允许第三方复用。
pub const DEFAULT_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";

const SCOPE: &str = "XboxLive.signin offline_access";
const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

/// 前端要展示给用户的信息：一串码 + 一个网址
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// 登录成功后拿到的正版身份
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct McProfile {
    pub uuid: String,
    pub name: String,
    pub access_token: String,
    pub refresh_token: String,
    /// 毫秒时间戳，到点前要刷新
    pub expires_at: u64,
    /// 皮肤地址（没有皮肤时为空）
    pub skin: String,
    /// 32x32 头像，没有皮肤时为空
    pub avatar: String,
}

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct XblResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    claims: XblClaims,
}

#[derive(Deserialize)]
struct XblClaims {
    xui: Vec<Xui>,
}

#[derive(Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Deserialize)]
struct McLoginResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize)]
struct McProfileResponse {
    id: String,
    name: String,
    #[serde(default)]
    skins: Vec<McSkin>,
}

#[derive(Deserialize)]
struct McSkin {
    url: String,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 第一步：申请设备码。用户要在浏览器里输入 user_code。
pub async fn start_device_code(
    client: &reqwest::Client,
    client_id: &str,
) -> Result<DeviceCode, String> {
    let response = client
        .post(DEVICE_CODE_URL)
        .form(&[("client_id", client_id), ("scope", SCOPE)])
        .send()
        .await
        .map_err(|e| format!("连不上微软登录服务: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("申请设备码失败（HTTP {status}）: {text}"));
    }

    let code: DeviceCodeResponse = response
        .json()
        .await
        .map_err(|e| format!("解析设备码失败: {e}"))?;

    Ok(DeviceCode {
        device_code: code.device_code,
        user_code: code.user_code,
        verification_uri: code.verification_uri,
        expires_in: code.expires_in,
        interval: code.interval.max(1),
    })
}

enum PollOutcome {
    Pending,
    Done(String, String, u64),
}

/// 第二步：轮询令牌。期间用户可能还没授权，返回 Pending 让前端继续转圈。
async fn poll_once(
    client: &reqwest::Client,
    client_id: &str,
    device_code: &str,
) -> Result<PollOutcome, String> {
    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
        ])
        .send()
        .await
        .map_err(|e| format!("轮询登录状态失败: {e}"))?;

    let token: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("解析令牌响应失败: {e}"))?;

    match token.error.as_deref() {
        None => {
            let access = token.access_token.ok_or("微软没有返回 access_token")?;
            let refresh = token.refresh_token.ok_or("微软没有返回 refresh_token")?;
            Ok(PollOutcome::Done(
                access,
                refresh,
                token.expires_in.unwrap_or(3600),
            ))
        }
        Some("authorization_pending") => Ok(PollOutcome::Pending),
        Some("slow_down") => Ok(PollOutcome::Pending),
        Some(other) => Err(format!(
            "登录失败：{}{}",
            other,
            token
                .error_description
                .map(|d| format!("（{d}）"))
                .unwrap_or_default()
        )),
    }
}

/// 用 Xbox Live + XSTS 换 Minecraft 的 access token，再拉档案。
async fn exchange(client: &reqwest::Client, msa_token: &str) -> Result<McProfile, String> {
    // 1) Xbox Live
    let xbl: XblResponse = client
        .post(XBL_URL)
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={msa_token}")
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .map_err(|e| format!("Xbox Live 认证失败: {e}"))?
        .json()
        .await
        .map_err(|e| format!("解析 Xbox Live 响应失败: {e}"))?;

    let uhs = xbl
        .claims
        .xui
        .first()
        .map(|x| x.uhs.clone())
        .ok_or("Xbox Live 没有返回用户标识（uhs）")?;

    // 2) XSTS
    let xsts_response = client
        .post(XSTS_URL)
        .json(&serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl.token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .map_err(|e| format!("XSTS 授权失败: {e}"))?;

    if !xsts_response.status().is_success() {
        let status = xsts_response.status();
        let text = xsts_response.text().await.unwrap_or_default();
        // 常见：没买过游戏 / 未成年账号
        return Err(format!(
            "XSTS 授权被拒（HTTP {status}）。常见原因：这个微软账号没有 Minecraft: Java 版，\
             或者账号未满 18 岁且没有家长同意。详情：{text}"
        ));
    }
    let xsts: XblResponse = xsts_response
        .json()
        .await
        .map_err(|e| format!("解析 XSTS 响应失败: {e}"))?;

    // 3) Minecraft Services
    let login: McLoginResponse = client
        .post(MC_LOGIN_URL)
        .json(&serde_json::json!({
            "identityToken": format!("XBL3.0 x={uhs};{}", xsts.token)
        }))
        .send()
        .await
        .map_err(|e| format!("换取 Minecraft 令牌失败: {e}"))?
        .json()
        .await
        .map_err(|e| format!("解析 Minecraft 令牌失败: {e}"))?;

    // 4) 档案（没有游戏的人这里会 404）
    let profile_response = client
        .get(MC_PROFILE_URL)
        .bearer_auth(&login.access_token)
        .send()
        .await
        .map_err(|e| format!("读取正版档案失败: {e}"))?;

    if profile_response.status().as_u16() == 404 {
        return Err("这个账号没有 Minecraft: Java 版（商城未购买）".into());
    }
    let profile: McProfileResponse = profile_response
        .json()
        .await
        .map_err(|e| format!("解析正版档案失败: {e}"))?;

    let skin = profile
        .skins
        .first()
        .map(|s| s.url.clone())
        .unwrap_or_default();

    Ok(McProfile {
        uuid: profile.id,
        name: profile.name,
        access_token: login.access_token,
        refresh_token: String::new(),
        expires_at: now_ms() + login.expires_in * 1000,
        avatar: skin.clone(),
        skin,
    })
}

/// 第二步（对外）：一次轮询。返回 `None` 表示用户还没在浏览器里授权。
pub async fn poll_device_code(
    client: &reqwest::Client,
    client_id: &str,
    device_code: &str,
) -> Result<Option<McProfile>, String> {
    match poll_once(client, client_id, device_code).await? {
        PollOutcome::Pending => Ok(None),
        PollOutcome::Done(msa_token, refresh_token, _expires) => {
            let mut profile = exchange(client, &msa_token).await?;
            profile.refresh_token = refresh_token;
            Ok(Some(profile))
        }
    }
}

/// 用 refresh token 换回新的正版令牌（重启后 / 过期后）。
pub async fn refresh(
    client: &reqwest::Client,
    client_id: &str,
    refresh_token: &str,
) -> Result<McProfile, String> {
    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ])
        .send()
        .await
        .map_err(|e| format!("刷新登录状态失败: {e}"))?;

    let token: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("解析刷新响应失败: {e}"))?;

    let access = token
        .access_token
        .ok_or_else(|| format!("登录已失效，请重新登录：{}", token.error.unwrap_or_default()))?;

    let mut profile = exchange(client, &access).await?;
    // 微软可能轮换 refresh token，没给就沿用旧的
    profile.refresh_token = token.refresh_token.unwrap_or_else(|| refresh_token.to_string());
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_client_id_is_a_uuid() {
        assert_eq!(DEFAULT_CLIENT_ID.len(), 36);
        assert_eq!(DEFAULT_CLIENT_ID.matches('-').count(), 4);
    }

    #[test]
    fn now_ms_is_sane() {
        // 2024-01-01 之后
        assert!(now_ms() > 1_704_067_200_000);
    }
}
