//! 服务器列表：读写游戏目录里的 `servers.dat`（未压缩的 NBT）。
//!
//! 游戏里「多人游戏」列表就是读这个文件，所以在启动器里加服务器，
//! 进游戏就能直接看到 —— 这比自己存一份 JSON 有用得多。
//!
//! 只实现服务器列表用到的几种 tag：Compound / List / String / Byte。

use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ServerEntry {
    pub name: String,
    /** `host` 或 `host:port` */
    pub ip: String,
    pub hidden: bool,
}

const TAG_END: u8 = 0;
const TAG_BYTE: u8 = 1;
const TAG_STRING: u8 = 8;
const TAG_LIST: u8 = 9;
const TAG_COMPOUND: u8 = 10;

/// 读 `servers.dat`；文件不存在或格式不对时返回空列表（游戏自己也是这行为）。
pub fn read_servers(dir: &Path) -> Vec<ServerEntry> {
    let Ok(bytes) = std::fs::read(dir.join("servers.dat")) else {
        return Vec::new();
    };
    parse(&bytes).unwrap_or_default()
}

/// 覆盖写 `servers.dat`。
pub fn write_servers(dir: &Path, servers: &[ServerEntry]) -> Result<(), String> {
    let bytes = build(servers);
    let path = dir.join("servers.dat");
    std::fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {e}"))?;
    std::fs::write(&path, bytes).map_err(|e| format!("写入 {} 失败: {e}", path.display()))
}

/// 加一台服务器（同名先删掉，避免重复）。
pub fn add_server(dir: &Path, name: &str, ip: &str) -> Result<Vec<ServerEntry>, String> {
    let mut list = read_servers(dir);
    list.retain(|s| s.name != name && s.ip != ip);
    list.push(ServerEntry {
        name: name.to_string(),
        ip: ip.to_string(),
        hidden: false,
    });
    write_servers(dir, &list)?;
    Ok(list)
}

/// 按 ip 删一台。
pub fn remove_server(dir: &Path, ip: &str) -> Result<Vec<ServerEntry>, String> {
    let mut list = read_servers(dir);
    list.retain(|s| s.ip != ip);
    write_servers(dir, &list)?;
    Ok(list)
}

// ── NBT 编解码（只覆盖 servers.dat 需要的部分）────────────────

fn build(servers: &[ServerEntry]) -> Vec<u8> {
    let mut out = vec![TAG_COMPOUND];
    write_string(&mut out, "");
    out.push(TAG_LIST);
    write_string(&mut out, "servers");
    out.push(TAG_COMPOUND);
    out.extend_from_slice(&(servers.len() as i32).to_be_bytes());
    for server in servers {
        // list 里的 compound 没有名字
        out.push(TAG_STRING);
        write_string(&mut out, "name");
        write_string(&mut out, &server.name);
        out.push(TAG_STRING);
        write_string(&mut out, "ip");
        write_string(&mut out, &server.ip);
        if server.hidden {
            out.push(TAG_BYTE);
            write_string(&mut out, "hidden");
            out.push(1);
        }
        out.push(TAG_END);
    }
    out.push(TAG_END);
    out
}

fn write_string(out: &mut Vec<u8>, text: &str) {
    let bytes = text.as_bytes();
    out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(bytes);
}

fn parse(bytes: &[u8]) -> Option<Vec<ServerEntry>> {
    let mut cur = Cursor { data: bytes, pos: 0 };
    // 根 compound
    if cur.u8()? != TAG_COMPOUND {
        return None;
    }
    let _root_name = cur.string()?;
    let mut servers = Vec::new();
    loop {
        let tag = cur.u8()?;
        if tag == TAG_END {
            break;
        }
        let name = cur.string()?;
        if tag == TAG_LIST && name == "servers" {
            let item_type = cur.u8()?;
            let count = cur.i32()?.max(0);
            for _ in 0..count {
                if item_type == TAG_COMPOUND {
                    servers.push(read_server(&mut cur)?);
                }
            }
        } else {
            skip(&mut cur, tag)?;
        }
    }
    Some(servers)
}

fn read_server(cur: &mut Cursor) -> Option<ServerEntry> {
    let mut entry = ServerEntry::default();
    loop {
        let tag = cur.u8()?;
        if tag == TAG_END {
            break;
        }
        let name = cur.string()?;
        match (tag, name.as_str()) {
            (TAG_STRING, "name") => entry.name = cur.string()?,
            (TAG_STRING, "ip") => entry.ip = cur.string()?,
            (TAG_BYTE, "hidden") => entry.hidden = cur.u8()? != 0,
            _ => skip(cur, tag)?,
        }
    }
    Some(entry)
}

/// 跳过不认识的 tag（保留原有字段不丢：游戏自己会写 icon、接受等）。
fn skip(cur: &mut Cursor, tag: u8) -> Option<()> {
    match tag {
        TAG_BYTE => {
            cur.bytes(1)?;
        }
        TAG_STRING => {
            cur.string()?;
        }
        TAG_LIST => {
            let item = cur.u8()?;
            let count = cur.i32()?.max(0) as usize;
            for _ in 0..count {
                skip_value(cur, item)?;
            }
        }
        TAG_COMPOUND => loop {
            let inner = cur.u8()?;
            if inner == TAG_END {
                break;
            }
            cur.string()?;
            skip_value(cur, inner)?;
        },
        // 其余数值类型按长度跳过
        _ => skip_value(cur, tag)?,
    }
    Some(())
}

fn skip_value(cur: &mut Cursor, tag: u8) -> Option<()> {
    let size = match tag {
        TAG_BYTE => 1,
        2 => 2,          // short
        3 => 4,          // int
        4 => 8,          // long
        5 => 4,          // float
        6 => 8,          // double
        7 => {           // byte array
            let len = cur.i32()?.max(0) as usize;
            len
        }
        8 => return skip(cur, TAG_STRING),
        9 => return skip(cur, TAG_LIST),
        10 => return skip(cur, TAG_COMPOUND),
        11 => {          // int array
            let len = cur.i32()?.max(0) as usize;
            len * 4
        }
        12 => {          // long array
            let len = cur.i32()?.max(0) as usize;
            len * 8
        }
        _ => return None,
    };
    cur.bytes(size)?;
    Some(())
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn u8(&mut self) -> Option<u8> {
        let value = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(value)
    }

    fn i32(&mut self) -> Option<i32> {
        let bytes = self.bytes(4)?;
        Some(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn bytes(&mut self, len: usize) -> Option<&[u8]> {
        let slice = self.data.get(self.pos..self.pos + len)?;
        self.pos += len;
        Some(slice)
    }

    fn string(&mut self) -> Option<String> {
        let len = {
            let bytes = self.bytes(2)?;
            u16::from_be_bytes([bytes[0], bytes[1]]) as usize
        };
        let raw = self.bytes(len)?;
        Some(String::from_utf8_lossy(raw).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_servers() {
        let dir = std::env::temp_dir().join(format!("dlp-servers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        assert!(read_servers(&dir).is_empty());

        let list = add_server(&dir, "朋友服", "play.example.com:25565").unwrap();
        assert_eq!(list.len(), 1);
        let read_back = read_servers(&dir);
        assert_eq!(read_back[0].name, "朋友服");
        assert_eq!(read_back[0].ip, "play.example.com:25565");

        add_server(&dir, "第二个", "1.2.3.4").unwrap();
        assert_eq!(read_servers(&dir).len(), 2);

        let left = remove_server(&dir, "1.2.3.4").unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(read_servers(&dir)[0].ip, "play.example.com:25565");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_ip_replaces_instead_of_duplicating() {
        let dir = std::env::temp_dir().join(format!("dlp-servers2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        add_server(&dir, "旧名", "same.host").unwrap();
        add_server(&dir, "新名", "same.host").unwrap();
        let list = read_servers(&dir);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "新名");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
