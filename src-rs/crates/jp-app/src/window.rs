//! 窗口装配：Mica 背景、标题、尺寸与启动位置（对应 `MainWindow.xaml(.cs)`）。
//!
//! 与原 XAML 版的两处有意差异，均在 README 说明：
//! 1. 保留系统标题栏，不做 `ExtendsContentIntoTitleBar` + 自定义 `TitleBar`——
//!    代码驱动 UI 需额外自绘可拖拽区与 caption 按钮命中区，收益不抵风险；
//! 2. 图标不走 `AppWindow.SetIcon("Assets/AppIcon.ico")`（依赖打包资源的相对 URI 解析），
//!    改为把同源转换出的 DIB .ico 编进 exe、`WM_SETICON` 挂到窗口，见 [`crate::icon`]。

use windows::Graphics::{PointInt32, SizeInt32};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
use crate::icon;
use windows_core::{HSTRING, Interface, Result as WinResult};
use winui3::Microsoft::UI::Windowing::{AppWindow, OverlappedPresenter};
use winui3::Microsoft::UI::Xaml::Media::MicaBackdrop;
use winui3::Microsoft::UI::Xaml::Window;

/// 屏幕几何用的浮点矩形（left/top/right/bottom）。
///
/// windows 0.62 没有现成的对应类型（`Windows::Foundation::Rect` 是 x/y/w/h 语义），
/// 放置判定的算术需要这套边角语义，就地定义。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// 与 `MainWindow.xaml` 一致：设计尺寸 1200×800，最小 900×600（逻辑像素）。
const PREFERRED_WIDTH: f32 = 1200.0;
const PREFERRED_HEIGHT: f32 = 800.0;
const MIN_WIDTH: f32 = 900.0;
const MIN_HEIGHT: f32 = 600.0;
const BASE_DPI: f32 = 96.0;

/// 启动位置决策：完全不在工作区内时按 100% 缩放重排，否则按当前缩放居中。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Placement {
    Visible { scale: f32 },
    OffScreen { reset_scale_to: f32 },
}

/// 窗口矩形与工作区矩形是否相交（RECT 左闭右开）。
///
/// 任一矩形为空（取几何失败）时保守判为可见：宁可位置不理想，也不要把
/// 一个本在屏幕上的窗口强行改成 100% 缩放。
pub fn decide_placement(window: Rect, work_area: Rect, dpi_scale: f32) -> Placement {
    let empty = window.right <= window.left
        || window.bottom <= window.top
        || work_area.right <= work_area.left
        || work_area.bottom <= work_area.top;
    if empty {
        return Placement::Visible { scale: dpi_scale };
    }

    let overlaps = window.left < work_area.right
        && window.right > work_area.left
        && window.top < work_area.bottom
        && window.bottom > work_area.top;

    if overlaps {
        Placement::Visible { scale: dpi_scale }
    } else {
        Placement::OffScreen { reset_scale_to: 1.0 }
    }
}

/// 在工作区内水平居中，顶部留出工作区高度的 10%（比严格垂直居中更顺手）。
pub fn centered(window: Rect, work_area: Rect) -> (i32, i32) {
    let width = (window.right - window.left).max(1.0);
    let left = work_area.left + ((work_area.right - work_area.left - width) / 2.0).max(0.0);
    let top = work_area.top + ((work_area.bottom - work_area.top) * 0.1).max(0.0);
    (left as i32, top as i32)
}

pub const fn dpi_scale_of(dpi: u32) -> f32 {
    if dpi == 0 {
        1.0
    } else {
        dpi as f32 / BASE_DPI
    }
}

/// 装配窗口：标题 → 设计尺寸 → 居中 → 最小尺寸 → Mica。
pub fn configure(window: &Window, title: &str) -> WinResult<()> {
    window.SetTitle(&HSTRING::from(title))?;

    let app_window = window.AppWindow()?;
    let hwnd = window_hwnd(&app_window)?;
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let scale = dpi_scale_of(dpi);
    app_window.Resize(size_at_scale(scale))?;

    let scale = center_on_first_show(&app_window, hwnd, scale)?;
    set_minimum_size(&app_window, scale)?;

    // 旧版 Windows 不支持 Mica：失败属正常降级，保留主题底色
    let _ = apply_mica(window);
    use_dark_title_bar(hwnd);
    icon::apply(hwnd, dpi.max(96));
    Ok(())
}

/// 标题栏走深色：正文配色是钉死的深色，亮色 caption 会形成断层。
/// 属性写失败（异常老的系统）只是回到亮标题栏，不影响功能。
fn use_dark_title_bar(hwnd: HWND) {
    let enabled: i32 = 1;
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            core::ptr::from_ref(&enabled).cast(),
            core::mem::size_of_val(&enabled) as u32,
        );
    }
}

/// 首次显示时居中；若窗口整个在可视区外，按 100% 缩放重排到最近显示器。
/// 返回最终生效的缩放，最小尺寸要按同一比例换算。
fn center_on_first_show(app_window: &AppWindow, hwnd: HWND, scale: f32) -> WinResult<f32> {
    let work_area = work_area_of(hwnd);
    let mut window_rect = window_rect_of(hwnd);

    let scale = match decide_placement(window_rect, work_area, scale) {
        Placement::Visible { scale } => scale,
        Placement::OffScreen { reset_scale_to } => {
            // 复位到设计尺寸后重新量一次，再按 100% 缩放摆放
            app_window.Resize(size_at_scale(reset_scale_to))?;
            window_rect = window_rect_of(hwnd);
            reset_scale_to
        }
    };

    let (left, top) = centered(window_rect, work_area);
    app_window.Move(PointInt32 { X: left, Y: top })?;
    Ok(scale)
}

/// `AppWindow.Resize` 收物理像素；`OverlappedPresenter` 的最小尺寸同样是物理像素。
fn set_minimum_size(app_window: &AppWindow, scale: f32) -> WinResult<()> {
    let presenter = app_window.Presenter()?.cast::<OverlappedPresenter>()?;
    presenter.SetPreferredMinimumWidth(&boxed_int((MIN_WIDTH * scale) as i32)?)?;
    presenter.SetPreferredMinimumHeight(&boxed_int((MIN_HEIGHT * scale) as i32)?)
}

fn boxed_int(value: i32) -> WinResult<windows::Foundation::IReference<i32>> {
    windows::Foundation::PropertyValue::CreateInt32(value)?.cast::<windows::Foundation::IReference<i32>>()
}

const fn size_at_scale(scale: f32) -> SizeInt32 {
    SizeInt32 {
        Width: (PREFERRED_WIDTH * scale) as i32,
        Height: (PREFERRED_HEIGHT * scale) as i32,
    }
}

/// `WindowId` 只是个值结构体，拿 HWND 必须过 FrameworkUdk 的 interop 入口。
fn window_hwnd(app_window: &AppWindow) -> WinResult<HWND> {
    crate::services::hwnd_of(app_window)
}

/// `GetWindowRect`：失败时返回空矩形，由 [`decide_placement`] 保守处理。
fn window_rect_of(hwnd: HWND) -> Rect {
    let mut native = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut native);
    }
    rect_of(&native)
}

/// 窗口所在显示器的工作区（物理像素）；取不到时退化为 1920×1080 全屏。
fn work_area_of(hwnd: HWND) -> Rect {
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: core::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };

    if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        let work = rect_of(&info.rcWork);
        if work.right > work.left && work.bottom > work.top {
            return work;
        }
    }

    Rect {
        left: 0.0,
        top: 0.0,
        right: 1920.0,
        bottom: 1080.0,
    }
}

const fn rect_of(rect: &RECT) -> Rect {
    Rect {
        left: rect.left as f32,
        top: rect.top as f32,
        right: rect.right as f32,
        bottom: rect.bottom as f32,
    }
}

/// Mica 背景：系统不支持时返回 `Err`，由调用方保留主题底色。
fn apply_mica(window: &Window) -> WinResult<()> {
    window.SetSystemBackdrop(&MicaBackdrop::new()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn visible_window_keeps_dpi_scale() {
        let placement = decide_placement(
            rect(100.0, 100.0, 900.0, 700.0),
            rect(0.0, 0.0, 1920.0, 1040.0),
            1.5,
        );
        assert_eq!(placement, Placement::Visible { scale: 1.5 });
    }

    #[test]
    fn partially_offscreen_window_stays_visible() {
        let placement = decide_placement(
            rect(-100.0, 200.0, 700.0, 800.0),
            rect(0.0, 0.0, 1920.0, 1040.0),
            2.0,
        );
        assert_eq!(placement, Placement::Visible { scale: 2.0 });
    }

    #[test]
    fn fully_offscreen_window_resets_to_100_percent() {
        let placement = decide_placement(
            rect(4000.0, 100.0, 4800.0, 700.0),
            rect(0.0, 0.0, 1920.0, 1040.0),
            1.5,
        );
        assert_eq!(placement, Placement::OffScreen { reset_scale_to: 1.0 });
    }

    #[test]
    fn unknown_geometry_is_treated_as_visible() {
        let placement = decide_placement(Rect::default(), rect(0.0, 0.0, 1920.0, 1040.0), 1.25);
        assert_eq!(placement, Placement::Visible { scale: 1.25 });
    }

    #[test]
    fn centering_clamps_to_work_area_and_offsets_top() {
        let (left, top) = centered(rect(0.0, 0.0, 1200.0, 800.0), rect(0.0, 0.0, 1920.0, 1040.0));
        assert_eq!((left, top), (360, 104));

        // 比工作区还宽时不往负方向跑
        let (left, _) = centered(rect(0.0, 0.0, 2400.0, 900.0), rect(0.0, 0.0, 1920.0, 1040.0));
        assert_eq!(left, 0);
    }

    #[test]
    fn zero_dpi_falls_back_to_one() {
        assert_eq!(dpi_scale_of(0), 1.0);
        assert_eq!(dpi_scale_of(96), 1.0);
        assert_eq!(dpi_scale_of(144), 1.5);
    }

    #[test]
    fn design_size_scales_with_dpi() {
        assert_eq!(
            size_at_scale(1.5),
            SizeInt32 {
                Width: 1800,
                Height: 1200
            }
        );
    }

    #[test]
    fn native_rect_converts_to_f32_rect() {
        let rect = rect_of(&RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        });
        assert_eq!(rect, Rect { left: 0.0, top: 0.0, right: 1920.0, bottom: 1040.0 });
    }
}
