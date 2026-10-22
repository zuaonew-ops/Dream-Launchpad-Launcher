//! 服务器状态查询：Minecraft Server List Ping 协议（1.7+）。
//!
//! 流程：TCP 连上 → 握手包（next_state=1）→ 状态请求 → 读回一段 JSON。
//! 协议细节：包长和包 id 都是 VarInt，JSON 长度是 VarInt + UTF-8。

use serde::Serialize;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub host: String,
    pub port: u16,
    pub online: bool,
    /// 服务器返回的 MOTD（已经去掉颜色代码）
    pub motd: String,
    pub players: u32,
    pub max_players: u32,
    /// 服务端版本名
    pub version: String,
    /// 握手到收到响应的毫秒数
    pub latency: u32,
    /// 连不上时的原因
    pub error: String,
}

/// 查一个服务器的状态。连不上不算错误 —— 返回 `online: false` 带原因。
pub async fn ping(host: &str, port: u16) -> ServerStatus {
    let mut status = ServerStatus {
        host: host.to_string(),
        port,
        online: false,
        motd: String::new(),
        players: 0,
        max_players: 0,
        version: String::new(),
        latency: 0,
        error: String::new(),
    };
    let started = Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(5), query(host, port)).await;
    status.latency = started.elapsed().as_millis() as u32;
    match result {
        Ok(Ok(info)) => {
            status.online = true;
            status.motd = strip_colors(&info.motd);
            status.players = info.players;
            status.max_players = info.max_players;
            status.version = info.version;
        }
        Ok(Err(e)) => status.error = e,
        Err(_) => status.error = "超时（5 秒）".into(),
    }
    status
}

struct Info {
    motd: String,
    players: u32,
    max_players: u32,
    version: String,
}

async fn query(host: &str, port: u16) -> Result<Info, String> {
    let mut stream = TcpStream::connect((host, port))
        .await
        .map_err(|e| format!("连不上 {host}:{port} —— {e}"))?;

    // 握手：protocol=-1（只问状态，不关心版本）、address、port、next_state=1
    let mut handshake = Vec::new();
    write_varint(&mut handshake, 0x00);
    write_varint(&mut handshake, -1); // 协议版本 -1 = 只问状态
    write_string(&mut handshake, host);
    handshake.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut handshake, 1);

    let mut packet = Vec::new();
    write_varint(&mut packet, handshake.len() as i32);
    packet.extend_from_slice(&handshake);
    stream
        .write_all(&packet)
        .await
        .map_err(|e| format!("发送握手失败: {e}"))?;

    // 状态请求：空包体，id=0
    let mut request = Vec::new();
    write_varint(&mut request, 1);
    write_varint(&mut request, 0x00);
    stream
        .write_all(&request)
        .await
        .map_err(|e| format!("发送请求失败: {e}"))?;

    let length = read_varint(&mut stream).await?;
    if length <= 0 || length > 8 * 1024 * 1024 {
        return Err(format!("响应长度异常: {length}"));
    }
    let mut body = vec![0u8; length as usize];
    stream
        .read_exact(&mut body)
        .await
        .map_err(|e| format!("读响应失败: {e}"))?;
    let mut cursor = Cursor::new(&body);
    let packet_id = cursor.varint()?;
    if packet_id != 0x00 {
        return Err(format!("意外的包 id: {packet_id}"));
    }
    let json_len = cursor.varint()?;
    let json = cursor.bytes(json_len as usize)?;
    let text = String::from_utf8_lossy(json);

    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("解析状态 JSON 失败: {e}"))?;
    let motd = match value.pointer("/description") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Object(_)) => {
            // 组件形式：{"text": "...", "extra": [...]}，只取 text 与 extra 的 text
            let mut out = String::new();
            collect_text(value.pointer("/description").unwrap(), &mut out);
            out
        }
        _ => String::new(),
    };
    Ok(Info {
        motd,
        players: value
            .pointer("/players/online")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        max_players: value
            .pointer("/players/max")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        version: value
            .pointer("/version/name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// 把聊天组件里的文字拼起来。
fn collect_text(value: &serde_json::Value, out: &mut String) {
    if let Some(text) = value.get("text").and_then(|v| v.as_str()) {
        out.push_str(text);
    }
    if let Some(extra) = value.get("extra").and_then(|v| v.as_array()) {
        for item in extra {
            collect_text(item, out);
        }
    }
}

/// 去掉 §a 这类颜色代码。
fn strip_colors(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out.trim().to_string()
}

fn write_varint(out: &mut Vec<u8>, value: i32) {
    let mut v = value as u32;
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if v == 0 {
            break;
        }
    }
}

fn write_string(out: &mut Vec<u8>, text: &str) {
    write_varint(out, text.len() as i32);
    out.extend_from_slice(text.as_bytes());
}

async fn read_varint(stream: &mut TcpStream) -> Result<i32, String> {
    let mut result: u32 = 0;
    let mut shift = 0;
    loop {
        let mut byte = [0u8; 1];
        stream
            .read_exact(&mut byte)
            .await
            .map_err(|e| format!("读 VarInt 失败: {e}"))?;
        result |= ((byte[0] & 0x7F) as u32) << shift;
        if byte[0] & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift > 28 {
            return Err("VarInt 太长".into());
        }
    }
    Ok(result as i32)
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn varint(&mut self) -> Result<i32, String> {
        let mut result: u32 = 0;
        let mut shift = 0;
        loop {
            let byte = *self.data.get(self.pos).ok_or("数据不够（VarInt）")?;
            self.pos += 1;
            result |= ((byte & 0x7F) as u32) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift > 28 {
                return Err("VarInt 太长".into());
            }
        }
        Ok(result as i32)
    }

    fn bytes(&mut self, len: usize) -> Result<&'a [u8], String> {
        let slice = self.data.get(self.pos..self.pos + len).ok_or("数据不够")?;
        self.pos += len;
        Ok(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_roundtrip() {
        for value in [0, 1, 127, 128, 255, 2097151, 2147483647] {
            let mut buf = Vec::new();
            write_varint(&mut buf, value);
            let mut cursor = Cursor::new(&buf);
            assert_eq!(cursor.varint().unwrap(), value);
        }
    }

    #[test]
    fn strips_color_codes() {
        assert_eq!(strip_colors("§aHello §rWorld"), "Hello World");
    }

    #[test]
    fn collects_component_text() {
        let value: serde_json::Value =
            serde_json::from_str(r#"{"text":"A","extra":[{"text":"B"},{"text":"C"}]}"#).unwrap();
        let mut out = String::new();
        collect_text(&value, &mut out);
        assert_eq!(out, "ABC");
    }
}
