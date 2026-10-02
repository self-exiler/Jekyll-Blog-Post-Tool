//! 应用图标：把 .NET 工程的 `Assets/AppIcon.ico` 编进 exe，运行时挂到窗口。
//!
//! 原版是 `AppWindow.SetIcon("Assets/AppIcon.ico")`（MainWindow.xaml.cs），依赖打包
//! 资源的相对 URI；未打包 + 代码构建的 Rust 版走 Win32 老路：解析 .ico 目录，
//! 用 DIB 段自己造 `HICON`，再 `WM_SETICON` 分别设标题栏小图与任务栏大图。
//!
//! 两处刻意的选择，都实测过：
//! - 资源 `assets/AppIcon.ico` 由 `scripts/gen-icon-asset.py` 从 .NET 那份转出。原件
//!   七个条目全是 PNG 压缩，而下面那条 `CreateIconFromResourceEx` 的坑让它连系统
//!   自带 .ico 的 BMP 条目都拒绝，所以转成免解码的 32bpp DIB；
//! - 本模块只负责**窗口**图标（标题栏、任务栏活动图标）。资源管理器、固定到开始屏幕/
//!   任务栏这些场合看的是 exe 里那份 `RT_GROUP_ICON`，由 `build.rs` 构建期挂进去。

/// 与 .NET 版同源；含 16/24/32/48/64 五档，覆盖到 200% 缩放的 caption 与 taskbar。
const APP_ICON: &[u8] = include_bytes!("../assets/AppIcon.ico");

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, DeleteObject, BI_RGB, BITMAPINFO, BITMAPINFOHEADER,
    DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
};
use windows::Win32::UI::HiDpi::GetSystemMetricsForDpi;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, SendMessageW, HICON, ICONINFO, ICON_BIG, ICON_SMALL, SM_CXICON,
    SM_CXSMICON, SYSTEM_METRICS_INDEX, WM_SETICON,
};

/// 标题栏小图标与任务栏大图在 100% 缩放下的兜底尺寸。
const SMALL_PX: u32 = 16;
const BIG_PX: u32 = 32;

/// .ico 里的一条目录项（已展开成可直接使用的宽高与切片区间）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconEntry {
    pub width: u32,
    pub height: u32,
    pub offset: usize,
    pub size: usize,
}

/// 解析 ICONDIR + ICONDIRENTRY 数组。
///
/// 头部残缺、条目越界、计数与实存不符都视为整体无效返回 `None`——
/// 宁可用系统默认图标，也不挂半张坏图。
pub fn parse_entries(ico: &[u8]) -> Option<Vec<IconEntry>> {
    let dir = ico.get(0..6)?;
    // 保留字段必须为 0，类型必须为 1（图标）
    if u16::from_le_bytes(dir[0..2].try_into().ok()?) != 0
        || u16::from_le_bytes(dir[2..4].try_into().ok()?) != 1
    {
        return None;
    }

    let count = u16::from_le_bytes(dir[4..6].try_into().ok()?) as usize;
    if count == 0 {
        return None;
    }

    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let record = ico.get(6 + index * 16..6 + (index + 1) * 16)?;
        let width = dimension(record[0]);
        let height = dimension(record[1]);
        let size = u32::from_le_bytes(record[8..12].try_into().ok()?) as usize;
        let offset = u32::from_le_bytes(record[12..16].try_into().ok()?) as usize;
        if size == 0 || offset.checked_add(size)? > ico.len() {
            return None;
        }
        entries.push(IconEntry {
            width,
            height,
            offset,
            size,
        });
    }
    Some(entries)
}

/// .ico 的宽高各用一个字节表示，0 即 256。
const fn dimension(byte: u8) -> u32 {
    if byte == 0 {
        256
    } else {
        byte as u32
    }
}

/// 挑选能覆盖请求尺寸的最小条目；全都比请求小时退而取最大的那张。
///
/// 降采样比放大清晰，所以宁可用 48px 画 32px，也不用 16px 撑到 32px。
pub fn pick_for_size(entries: &[IconEntry], requested: u32) -> Option<&IconEntry> {
    entries
        .iter()
        .filter(|entry| entry.width >= requested && entry.height >= requested)
        .min_by_key(|entry| entry.width * entry.height)
        .or_else(|| entries.iter().max_by_key(|entry| entry.width * entry.height))
}

/// 取出条目里 ICONIMAGE 的像素体，统一成顶向下的 BGRA 行。
///
/// 返回 `(像素, 每行字节数)`。`biHeight` 为 `2 * 高` 表示自底向上，需要倒序。
fn pixel_rows(entry: &IconEntry, ico: &[u8]) -> Option<(Vec<u8>, usize)> {
    let header = ico.get(entry.offset..entry.offset + 40)?;
    let dib_size = u32::from_le_bytes(header[0..4].try_into().ok()?) as usize;
    let raw_height = i32::from_le_bytes(header[8..12].try_into().ok()?);
    let height = raw_height.unsigned_abs() / 2;
    let bpp = u16::from_le_bytes(header[14..16].try_into().ok()?);
    if dib_size < 40 || height != entry.height || bpp != 32 {
        return None;
    }

    let body = ico.get(entry.offset + dib_size..entry.offset + entry.size)?;
    let stride = entry.width as usize * 4;
    let plane = stride * height as usize;
    if body.len() < plane {
        return None;
    }

    let mut rows = vec![0u8; plane];
    for index in 0..height as usize {
        // 正高度：磁盘首行是底行；负高度：已经是顶向下
        let source = if raw_height > 0 {
            height as usize - 1 - index
        } else {
            index
        };
        rows[index * stride..(index + 1) * stride]
            .copy_from_slice(&body[source * stride..(source + 1) * stride]);
    }
    Some((rows, stride))
}

/// 把应用图标挂到窗口：小图给标题栏，大图给任务栏。
///
/// 任何一步失败都只是维持系统默认图标，不影响窗口功能。图标句柄随进程存活、
/// 不销毁——窗口仍在引用它们。
pub fn apply(hwnd: HWND, dpi: u32) {
    use windows::Win32::Foundation::{LPARAM, WPARAM};

    let Some(entries) = parse_entries(APP_ICON) else {
        return;
    };

    for (metric, fallback, which) in [
        (SM_CXSMICON, SMALL_PX, ICON_SMALL),
        (SM_CXICON, BIG_PX, ICON_BIG),
    ] {
        let want = icon_size(metric, dpi, fallback);
        let Some(icon) = pick_for_size(&entries, want).and_then(|entry| build(entry)) else {
            continue;
        };
        unsafe {
            let _ = SendMessageW(
                hwnd,
                WM_SETICON,
                Some(WPARAM(which as usize)),
                Some(LPARAM(icon.0 as isize)),
            );
        }
    }
}

/// 该 DPI 下系统建议的图标边长；问不到就按 96 基准缩放。
fn icon_size(metric: SYSTEM_METRICS_INDEX, dpi: u32, fallback: u32) -> u32 {
    let want = unsafe { GetSystemMetricsForDpi(metric, dpi.max(96)) };
    if want > 0 {
        want as u32
    } else {
        (fallback as f32 * crate::window::dpi_scale_of(dpi)) as u32
    }
}

/// 由条目造 `HICON`。
///
/// 不用 `CreateIconFromResourceEx`：实测本机（Win11 26100 + WinAppSDK 2.5）对 PNG
/// 与 BMP 条目一律返回 NULL，连系统自带 .ico 也一样；`CreateDIBSection` +
/// `CreateIconIndirect` 这条路径可用，像素由 [`pixel_rows`] 自己摆。
fn build(entry: &IconEntry) -> Option<HICON> {
    let (pixels, _) = pixel_rows(entry, APP_ICON)?;
    let side = entry.width;

    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: side as i32,
            biHeight: -(side as i32), // 负值 = 顶向下，与 pixels 的行序一致
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut bits: *mut core::ffi::c_void = core::ptr::null_mut();
    let Ok(color) = (unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) })
    else {
        return None;
    };
    if bits.is_null() {
        free(color);
        return None;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len());
    }

    // AND 掩码全零：32bpp 图标的透明度由 alpha 通道承担
    let mask_stride = ((side + 31) / 32) * 4;
    let mask = vec![0u8; (mask_stride * side) as usize];
    let mask_bitmap = unsafe { CreateBitmap(side as i32, side as i32, 1, 1, Some(mask.as_ptr().cast())) };
    if mask_bitmap.is_invalid() {
        free(color);
        return None;
    }

    let icon = unsafe {
        CreateIconIndirect(&ICONINFO {
            fIcon: windows_core::BOOL(1),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask_bitmap,
            hbmColor: color,
        })
    };

    free(mask_bitmap);
    free(color);
    icon.ok()
}

/// `CreateIconIndirect` 已经把位图内容复制进图标，这两个句柄可以立刻释放。
fn free(bitmap: HBITMAP) {
    if !bitmap.is_invalid() {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造最小合法 .ico：header + n 条目录项 + 每条一段 DIB 填充体。
    fn synthetic_ico(sizes: &[u32]) -> Vec<u8> {
        let mut plan = Vec::with_capacity(sizes.len());
        let mut offset = 6 + sizes.len() * 16;
        for &px in sizes {
            let body = 40 + (px as usize * px as usize * 4) + (px as usize * 4);
            plan.push((px, offset, body));
            offset += body;
        }

        let mut file = vec![0, 0, 1, 0, sizes.len() as u8, 0];
        file.resize(6 + plan.len() * 16, 0);
        for (index, (px, offset, body)) in plan.iter().enumerate() {
            let record = 6 + index * 16;
            let byte = if *px >= 256 { 0 } else { *px as u8 };
            file[record] = byte;
            file[record + 1] = byte;
            file[record + 8..record + 12].copy_from_slice(&(*body as u32).to_le_bytes());
            file[record + 12..record + 16].copy_from_slice(&(*offset as u32).to_le_bytes());

            // 图像体：BITMAPINFOHEADER（biHeight = 2 * 高，自底向上）+ 像素 + 掩码
            let mut dib = vec![0u8; *body];
            dib[0..4].copy_from_slice(&40u32.to_le_bytes());
            dib[4..8].copy_from_slice(&(*px as i32).to_le_bytes());
            dib[8..12].copy_from_slice(&((*px * 2) as i32).to_le_bytes());
            dib[12..14].copy_from_slice(&1u16.to_le_bytes());
            dib[14..16].copy_from_slice(&32u16.to_le_bytes());
            file.extend_from_slice(&dib);
        }
        file
    }

    #[test]
    fn real_app_icon_parses() {
        let entries = parse_entries(APP_ICON).expect("AppIcon.ico 应能解析");
        assert!(!entries.is_empty());
        for entry in &entries {
            assert!(entry.width >= 16 && entry.width <= 256);
            assert_eq!(entry.width, entry.height);
            assert!(entry.offset + entry.size <= APP_ICON.len());
        }
    }

    #[test]
    fn header_must_be_an_icon_directory() {
        assert_eq!(parse_entries(&[0u8; 64]), None); // 类型字段不是 1
        assert_eq!(parse_entries(&[]), None);
        assert_eq!(parse_entries(&[0, 0, 1, 0, 0, 0]), None); // count = 0
    }

    #[test]
    fn truncated_image_body_is_rejected() {
        let mut file = synthetic_ico(&[32]);
        let len = file.len();
        file.truncate(len - 1);
        assert_eq!(parse_entries(&file), None);
    }

    #[test]
    fn zero_byte_dimension_means_256() {
        let entries = parse_entries(&synthetic_ico(&[256])).unwrap();
        assert_eq!(entries[0].width, 256);
        assert_eq!(entries[0].height, 256);
    }

    #[test]
    fn smallest_covering_entry_wins() {
        let file = synthetic_ico(&[16, 48, 256]);
        let entries = parse_entries(&file).unwrap();
        assert_eq!(pick_for_size(&entries, 16).unwrap().width, 16);
        // 没有 32px 条目时降采样 48，而不是放大 16
        assert_eq!(pick_for_size(&entries, 32).unwrap().width, 48);
        assert_eq!(pick_for_size(&entries, 200).unwrap().width, 256);
        // 全都比请求小时取最大的那张
        assert_eq!(pick_for_size(&entries, 300).unwrap().width, 256);
        assert_eq!(pick_for_size(&[] as &[IconEntry], 32), None);
    }

    #[test]
    fn real_icon_covers_small_and_large_requests() {
        let entries = parse_entries(APP_ICON).unwrap();
        assert_eq!(pick_for_size(&entries, SMALL_PX).unwrap().width, 16);
        assert_eq!(pick_for_size(&entries, BIG_PX).unwrap().width, 32);
        // 200% 缩放下 caption 要 32、taskbar 要 64，资源里都得有原生条目
        assert_eq!(pick_for_size(&entries, 64).unwrap().width, 64);
    }

    #[test]
    fn every_real_entry_exposes_top_down_pixels() {
        let entries = parse_entries(APP_ICON).unwrap();
        for entry in entries {
            let (pixels, stride) = pixel_rows(&entry, APP_ICON).unwrap_or_else(|| {
                panic!("{}px 条目应能取出 32bpp 像素体", entry.width)
            });
            assert_eq!(stride, entry.width as usize * 4);
            assert_eq!(pixels.len(), stride * entry.height as usize);
        }
    }
}
