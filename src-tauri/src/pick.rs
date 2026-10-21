//! 原生「选文件」对话框：只用 Windows API，不引 tauri-plugin-dialog。

use std::path::PathBuf;

/// 打开文件选择框，返回选中的绝对路径（取消返回 None）。
#[cfg(windows)]
pub fn pick_file(title: &str, filter: &str, extension: &str) -> Result<Option<PathBuf>, String> {
    use windows_sys::Win32::UI::Controls::Dialogs::{GetOpenFileNameW, OPENFILENAMEW};

    // 双 NUL 结尾的过滤器：显示名\0模式\0...\0\0
    let mut filter_wide: Vec<u16> = Vec::new();
    for part in [filter, &format!("*.{extension}")] {
        filter_wide.extend(part.encode_utf16());
        filter_wide.push(0);
    }
    filter_wide.push(0);
    let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();

    let mut buffer = vec![0u16; 32768];
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.lpstrFilter = filter_wide.as_ptr();
    ofn.lpstrFile = buffer.as_mut_ptr();
    ofn.nMaxFile = buffer.len() as u32;
    ofn.lpstrTitle = title_wide.as_ptr();
    ofn.Flags = 0x0000_1000 | 0x0000_0008; // OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST

    let ok = unsafe { GetOpenFileNameW(&mut ofn) };
    if ok == 0 {
        return Ok(None);
    }
    let len = buffer.iter().position(|c| *c == 0).unwrap_or(0);
    let path = String::from_utf16_lossy(&buffer[..len]);
    if path.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(path)))
    }
}

#[cfg(not(windows))]
pub fn pick_file(_title: &str, _filter: &str, _extension: &str) -> Result<Option<PathBuf>, String> {
    Err("这个平台还没有文件选择框，请手动填路径".into())
}
