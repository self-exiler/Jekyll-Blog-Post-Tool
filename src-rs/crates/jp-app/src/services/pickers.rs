//! 文件与目录选择器：`Windows.Storage.Pickers` 必须先经 `IInitializeWithWindow` 绑定宿主窗口。
//!
//! 该接口的 IID 取 Shell 那份（`{3E68D4BD-…}`，`windows` 投影在 `Win32::UI::Shell` 下导出）。
//! 网上常见的 `WinRT.Interop.InitializeWithWindow`（`{00000112-0000-0031-…}`）选择器**不认**，
//! `cast` 直接 `E_NOINTERFACE`，表现为按钮点了没反应（错误只进 crash.log）。
//!
//! 与对话框服务同一套线程约定：公开函数在工作线程上阻塞，UI 线程只负责起选择器并登记完成回调。

use std::path::PathBuf;
use std::sync::mpsc::Sender;

use windows::Storage::Pickers::{FileOpenPicker, FolderPicker, PickerLocationId};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::IInitializeWithWindow;
use windows_core::{HSTRING, Interface, Result as WinResult};

use crate::runtime::{append_crash_log, ask_ui};
use super::hwnd;

/// 绑定失败即无法归属窗口（系统会直接拒绝显示），调用方按「取消」处理并留日志。
fn bind_to_window(picker: &impl Interface, owner: HWND) -> WinResult<()> {
    let initializer: IInitializeWithWindow = picker.cast()?;
    // SAFETY: `owner` 来自当前 `AppWindow`，选择器对象已激活
    unsafe { initializer.Initialize(owner) }
}

fn owner_hwnd() -> WinResult<HWND> {
    hwnd().ok_or_else(windows_core::Error::empty)
}

fn to_path(path: HSTRING) -> PathBuf {
    PathBuf::from(path.to_string())
}

/// 选择博客项目根目录（对偶 `PickFolderAsync`）。
pub fn pick_folder() -> Option<PathBuf> {
    ask_ui(move |tx: Sender<Option<PathBuf>>| {
        let started = (|| -> WinResult<_> {
            let picker = FolderPicker::new()?;
            picker.SetSuggestedStartLocation(PickerLocationId::DocumentsLibrary)?;
            picker.FileTypeFilter()?.Append(&HSTRING::from("*"))?;
            bind_to_window(&picker, owner_hwnd()?)?;
            picker.PickSingleFolderAsync()
        })();

        let operation = match started {
            Ok(operation) => operation,
            Err(error) => {
                append_crash_log(&format!("目录选择器启动失败: {error}"));
                let _ = tx.send(None);
                return;
            }
        };

        let keep_alive = operation.clone();
        let done_tx = tx.clone();
        if let Err(error) = operation.when(move |result| {
            let _ = keep_alive;
            let selected = result
                .ok()
                .and_then(|folder| folder.Path().ok())
                .map(to_path);
            let _ = done_tx.send(selected);
        }) {
            append_crash_log(&format!("目录选择器回调登记失败: {error}"));
            let _ = tx.send(None);
        }
    })
    .unwrap_or(None)
}

/// 选择单个博文文件（对偶 `PickFileAsync`，只允许 `.md`）。
pub fn pick_markdown_file() -> Option<PathBuf> {
    ask_ui(move |tx: Sender<Option<PathBuf>>| {
        let started = (|| -> WinResult<_> {
            let picker = FileOpenPicker::new()?;
            picker.SetSuggestedStartLocation(PickerLocationId::DocumentsLibrary)?;
            picker.FileTypeFilter()?.Append(&HSTRING::from(".md"))?;
            bind_to_window(&picker, owner_hwnd()?)?;
            picker.PickSingleFileAsync()
        })();

        let operation = match started {
            Ok(operation) => operation,
            Err(error) => {
                append_crash_log(&format!("文件选择器启动失败: {error}"));
                let _ = tx.send(None);
                return;
            }
        };

        let keep_alive = operation.clone();
        let done_tx = tx.clone();
        if let Err(error) = operation.when(move |result| {
            let _ = keep_alive;
            let selected = result
                .ok()
                .and_then(|file| file.Path().ok())
                .map(to_path);
            let _ = done_tx.send(selected);
        }) {
            append_crash_log(&format!("文件选择器回调登记失败: {error}"));
            let _ = tx.send(None);
        }
    })
    .unwrap_or(None)
}

/// 多选图片（对偶 `PickFilesAsync`）；取消或出错返回空表。
pub fn pick_images(extensions: &[&str]) -> Vec<PathBuf> {
    let extensions: Vec<String> = extensions.iter().map(|extension| (*extension).to_owned()).collect();

    ask_ui(move |tx: Sender<Vec<PathBuf>>| {
        let started = (|| -> WinResult<_> {
            let picker = FileOpenPicker::new()?;
            picker.SetSuggestedStartLocation(PickerLocationId::PicturesLibrary)?;
            let filter = picker.FileTypeFilter()?;
            for extension in &extensions {
                filter.Append(&HSTRING::from(extension.as_str()))?;
            }
            bind_to_window(&picker, owner_hwnd()?)?;
            picker.PickMultipleFilesAsync()
        })();

        let operation = match started {
            Ok(operation) => operation,
            Err(error) => {
                append_crash_log(&format!("图片选择器启动失败: {error}"));
                let _ = tx.send(Vec::new());
                return;
            }
        };

        let keep_alive = operation.clone();
        let done_tx = tx.clone();
        if let Err(error) = operation.when(move |result| {
            let _ = keep_alive;
            let mut paths = Vec::new();

            if let Ok(files) = result {
                if let Ok(size) = files.Size() {
                    for index in 0..size {
                        if let Some(path) = files.GetAt(index).ok().and_then(|file| file.Path().ok()) {
                            paths.push(to_path(path));
                        }
                    }
                }
            }
            let _ = done_tx.send(paths);
        }) {
            append_crash_log(&format!("图片选择器回调登记失败: {error}"));
            let _ = tx.send(Vec::new());
        }
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pickers_answer_cancel_without_ui_thread() {
        // 没有 UI 线程时按取消处理，绝不阻塞
        assert_eq!(pick_folder(), None);
        assert_eq!(pick_markdown_file(), None);
        assert!(pick_images(&[".png"]).is_empty());
    }

    #[test]
    fn path_conversion_keeps_windows_separators() {
        let path = to_path(HSTRING::from("C:\\blog\\_posts\\2026-07-28-demo.md"));
        assert_eq!(path.file_name().unwrap().to_string_lossy(), "2026-07-28-demo.md");
    }
}
