//! 存档详情：读 `level.dat` 里的世界名、种子、游戏模式、游玩时间。
//!
//! `level.dat` 是 **gzip 压缩的 NBT**，结构和 `servers.dat` 那种裸 NBT 一样，
//! 只是外层多一层 gzip，字段类型也多（long / int / byte）。

use serde::Serialize;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize)]
pub struct LevelInfo {
    pub name: String,
    pub seed: i64,
    /// 0 生存 / 1 创造 / 2 冒险 / 3 旁观
    pub game_type: i32,
    /// 1 和平 / 2 简单 / 3 普通 / 4 困难
    pub difficulty: i32,
    pub hardcore: bool,
    pub day_time: i64,
    pub last_played: i64,
    /// 最后用哪个版本存的
    pub version: String,
    /// 世界里走过的游戏刻（20 刻 = 1 秒）
    pub played_ticks: i64,
}

/// 读一个存档的详情；读不出来返回 None。
pub fn read(save_dir: &Path) -> Option<LevelInfo> {
    let bytes = std::fs::read(save_dir.join("level.dat")).ok()?;
    read_bytes(&bytes)
}

/// 解析一份 gzip 压缩的 `level.dat` 内容。
pub fn read_bytes(bytes: &[u8]) -> Option<LevelInfo> {
    let mut decoder = flate2::read::GzDecoder::new(bytes);
    let mut raw = Vec::new();
    decoder.read_to_end(&mut raw).ok()?;

    let mut cursor = Cursor::new(&raw);
    // 根是 TAG_Compound，名字通常空
    if cursor.u8()? != TAG_COMPOUND {
        return None;
    }
    cursor.string()?;
    let mut data = None;
    loop {
        let tag = cursor.u8()?;
        if tag == TAG_END {
            break;
        }
        let name = cursor.string()?;
        if tag == TAG_COMPOUND && name == "Data" {
            data = Some(cursor.compound()?);
        } else {
            cursor.skip(tag)?;
        }
    }

    let data = data?;
    let mut info = LevelInfo::default();
    info.name = data.string_of("LevelName").unwrap_or_default();
    info.seed = data.long_of("RandomSeed").unwrap_or(0);
    info.game_type = data.int_of("GameType").unwrap_or(0);
    info.difficulty = data.byte_of("Difficulty").unwrap_or(2) as i32;
    info.hardcore = data.byte_of("hardcore").unwrap_or(0) != 0;
    info.day_time = data.long_of("DayTime").unwrap_or(0);
    info.last_played = data.long_of("LastPlayed").unwrap_or(0);
    // 1.16+ 在世界自己的 Version 里
    if let Some(version) = data.compound_of("Version") {
        info.version = version.string_of("Name").unwrap_or_default();
    }
    // 游玩时长在 Player 里的 SpawnX 旁边？不 —— 是 Data.Player 之外没有；
    // 1.13+ 用 Data.LevelName 同级的「Player」里没有时长，
    // 这里用 DayTime 兜底：它不是游玩时长，所以留 0 表示未知
    info.played_ticks = 0;
    Some(info)
}

/// 一个解析出来的 compound：字段名 → 值。
#[derive(Default)]
struct Compound {
    strings: std::collections::HashMap<String, String>,
    ints: std::collections::HashMap<String, i32>,
    longs: std::collections::HashMap<String, i64>,
    bytes: std::collections::HashMap<String, i8>,
    compounds: std::collections::HashMap<String, Compound>,
}

impl Compound {
    fn string_of(&self, key: &str) -> Option<String> {
        self.strings.get(key).cloned()
    }
    fn int_of(&self, key: &str) -> Option<i32> {
        self.ints.get(key).copied()
    }
    fn long_of(&self, key: &str) -> Option<i64> {
        self.longs.get(key).copied()
    }
    fn byte_of(&self, key: &str) -> Option<i8> {
        self.bytes.get(key).copied()
    }
    fn compound_of(&self, key: &str) -> Option<&Compound> {
        self.compounds.get(key)
    }
}

const TAG_END: u8 = 0;
const TAG_BYTE: u8 = 1;
const TAG_SHORT: u8 = 2;
const TAG_INT: u8 = 3;
const TAG_LONG: u8 = 4;
const TAG_FLOAT: u8 = 5;
const TAG_DOUBLE: u8 = 6;
const TAG_BYTE_ARRAY: u8 = 7;
const TAG_STRING: u8 = 8;
const TAG_LIST: u8 = 9;
const TAG_COMPOUND: u8 = 10;
const TAG_INT_ARRAY: u8 = 11;
const TAG_LONG_ARRAY: u8 = 12;

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn u8(&mut self) -> Option<u8> {
        let v = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(v)
    }

    fn bytes(&mut self, len: usize) -> Option<&'a [u8]> {
        let slice = self.data.get(self.pos..self.pos + len)?;
        self.pos += len;
        Some(slice)
    }

    fn i16(&mut self) -> Option<i16> {
        let b = self.bytes(2)?;
        Some(i16::from_be_bytes([b[0], b[1]]))
    }

    fn i32(&mut self) -> Option<i32> {
        let b = self.bytes(4)?;
        Some(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i64(&mut self) -> Option<i64> {
        let b = self.bytes(8)?;
        Some(i64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn string(&mut self) -> Option<String> {
        let len = self.i16()? as usize;
        let raw = self.bytes(len)?;
        Some(String::from_utf8_lossy(raw).to_string())
    }

    /// 读一个 compound 的所有字段（不认识的类型跳过）。
    fn compound(&mut self) -> Option<Compound> {
        let mut out = Compound::default();
        loop {
            let tag = self.u8()?;
            if tag == TAG_END {
                break;
            }
            let name = self.string()?;
            match tag {
                TAG_BYTE => {
                    out.bytes.insert(name, self.u8()? as i8);
                }
                TAG_SHORT => {
                    self.i16()?;
                }
                TAG_INT => {
                    out.ints.insert(name, self.i32()?);
                }
                TAG_LONG => {
                    out.longs.insert(name, self.i64()?);
                }
                TAG_FLOAT => {
                    self.bytes(4)?;
                }
                TAG_DOUBLE => {
                    self.bytes(8)?;
                }
                TAG_STRING => {
                    out.strings.insert(name, self.string()?);
                }
                TAG_COMPOUND => {
                    out.compounds.insert(name, self.compound()?);
                }
                other => self.skip(other)?,
            }
        }
        Some(out)
    }

    /// 跳过不关心的值。
    fn skip(&mut self, tag: u8) -> Option<()> {
        match tag {
            TAG_BYTE => {
                self.bytes(1)?;
            }
            TAG_SHORT => {
                self.bytes(2)?;
            }
            TAG_INT | TAG_FLOAT => {
                self.bytes(4)?;
            }
            TAG_LONG | TAG_DOUBLE => {
                self.bytes(8)?;
            }
            TAG_BYTE_ARRAY => {
                let len = self.i32()?.max(0) as usize;
                self.bytes(len)?;
            }
            TAG_INT_ARRAY => {
                let len = self.i32()?.max(0) as usize;
                self.bytes(len * 4)?;
            }
            TAG_LONG_ARRAY => {
                let len = self.i32()?.max(0) as usize;
                self.bytes(len * 8)?;
            }
            TAG_STRING => {
                self.string()?;
            }
            TAG_LIST => {
                let item = self.u8()?;
                let count = self.i32()?.max(0) as usize;
                for _ in 0..count {
                    self.skip(item)?;
                }
            }
            TAG_COMPOUND => {
                loop {
                    let inner = self.u8()?;
                    if inner == TAG_END {
                        break;
                    }
                    self.string()?;
                    self.skip(inner)?;
                }
            }
            _ => return None,
        }
        Some(())
    }
}

/// 把「刻」换算成「几小时几分钟」，界面直接显示。
pub fn format_ticks(ticks: i64) -> String {
    if ticks <= 0 {
        return "未知".into();
    }
    let seconds = ticks / 20;
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours} 小时 {minutes} 分")
    } else {
        format!("{minutes} 分钟")
    }
}

/// 游戏模式名。
pub fn game_type_name(value: i32) -> &'static str {
    match value {
        0 => "生存",
        1 => "创造",
        2 => "冒险",
        3 => "旁观",
        _ => "未知",
    }
}

/// 难度名。
pub fn difficulty_name(value: i32) -> &'static str {
    match value {
        1 => "和平",
        2 => "简单",
        3 => "普通",
        4 => "困难",

        _ => "未知",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// 造一个最小 level.dat（gzip + NBT），字段够 read() 用。
    fn make_level_dat(path: &std::path::Path, name: &str, seed: i64) {
        let mut nbt: Vec<u8> = Vec::new();
        nbt.push(TAG_COMPOUND);
        write_str(&mut nbt, "");
        // Data
        nbt.push(TAG_COMPOUND);
        write_str(&mut nbt, "Data");
        nbt.push(TAG_STRING);
        write_str(&mut nbt, "LevelName");
        write_str(&mut nbt, name);
        nbt.push(TAG_LONG);
        write_str(&mut nbt, "RandomSeed");
        nbt.extend_from_slice(&seed.to_be_bytes());
        nbt.push(TAG_INT);
        write_str(&mut nbt, "GameType");
        nbt.extend_from_slice(&1i32.to_be_bytes());
        nbt.push(TAG_BYTE);
        write_str(&mut nbt, "hardcore");
        nbt.push(1);
        nbt.push(TAG_END); // Data 结束
        nbt.push(TAG_END); // 根结束

        let mut encoder = flate2::write::GzEncoder::new(
            std::fs::File::create(path).unwrap(),
            flate2::Compression::default(),
        );
        encoder.write_all(&nbt).unwrap();
        encoder.finish().unwrap();
    }

    fn write_str(out: &mut Vec<u8>, text: &str) {
        out.extend_from_slice(&(text.len() as i16).to_be_bytes());
        out.extend_from_slice(text.as_bytes());
    }

    #[test]
    fn reads_level_dat() {
        let dir = crate::testdir::temp_dir("dlp-level");
        std::fs::create_dir_all(&dir).unwrap();
        make_level_dat(&dir.join("level.dat"), "我的世界", 1234567);

        let info = read(&dir).expect("应该能读出来");
        assert_eq!(info.name, "我的世界");
        assert_eq!(info.seed, 1234567);
        assert_eq!(info.game_type, 1);
        assert!(info.hardcore);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_is_none() {
        let dir = crate::testdir::temp_dir("dlp-level2");
        assert!(read(&dir).is_none());
    }

    #[test]
    fn names_are_human_readable() {
        assert_eq!(game_type_name(0), "生存");
        assert_eq!(game_type_name(1), "创造");
        assert_eq!(difficulty_name(4), "困难");
        assert_eq!(format_ticks(0), "未知");
        assert_eq!(format_ticks(20 * 60 * 90), "1 小时 30 分");
        assert_eq!(format_ticks(20 * 60 * 5), "5 分钟");
    }
}
