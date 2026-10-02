//! 对话框服务：把 `ContentDialog` 包成阻塞调用，供工作线程在执行动作时提问。
//!
//! 调用约定：这些函数只能在工作线程上调用（`actions` 里所有命令都先派发到工作线程）。
//! 内部经 `runtime::ask_ui` 把对话框投到 UI 线程，然后阻塞自己等回答；
//! UI 线程注册完 `ShowAsync` 的完成回调就立刻返回，继续跑消息循环，所以不会互相等死。

use std::sync::mpsc::Sender;

use windows::Foundation::PropertyValue;
use windows_core::{HSTRING, Result as WinResult};
use winui3::Microsoft::UI::Xaml::Controls::{ContentDialog, ContentDialogButton, ContentDialogResult};

use jp_application::posts::{ConflictResolutionKind, ConflictResult, SavePrompts};

use crate::runtime::{append_crash_log, ask_ui};
use super::xaml_root;

/// 标题与正文都是 `IInspectable` 形参：投影不接受裸字符串，按 WinRT 惯例先装箱。
fn boxed(text: &str) -> WinResult<windows_core::IInspectable> {
    PropertyValue::CreateString(&HSTRING::from(text))
}

fn build(
    title: &str,
    message: &str,
    primary: Option<&str>,
    secondary: Option<&str>,
    close: Option<&str>,
) -> WinResult<ContentDialog> {
    let dialog = ContentDialog::new()?;
    dialog.SetTitle(&boxed(title)?)?;
    dialog.SetContent(&boxed(message)?)?;

    if let Some(text) = primary {
        dialog.SetPrimaryButtonText(&HSTRING::from(text))?;
    }
    if let Some(text) = secondary {
        dialog.SetSecondaryButtonText(&HSTRING::from(text))?;
    }
    if let Some(text) = close {
        dialog.SetCloseButtonText(&HSTRING::from(text))?;
    }

    Ok(dialog)
}

/// 显示对话框并阻塞等待点击结果；显示失败按「用户取消」处理，同时留崩溃日志（不静默吞错）。
fn show_blocking<F>(dialog_builder: F) -> ContentDialogResult
where
    F: FnOnce() -> WinResult<ContentDialog> + Send + 'static,
{
    ask_ui(move |tx: Sender<ContentDialogResult>| {
        let cancel_tx = tx.clone();
        let opened = dialog_builder().and_then(|dialog| {
            dialog.SetXamlRoot(&xaml_root()?)?;
            dialog.ShowAsync()
        });

        match opened {
            Ok(operation) => {
                // 回调触发前持有操作引用，避免唯一的强引用提前释放
                let keep_alive = operation.clone();
                if let Err(error) = operation.when(move |result| {
                    let _ = keep_alive;
                    let _ = tx.send(result.unwrap_or(ContentDialogResult::None));
                }) {
                    give_up("对话框订阅失败", &error, cancel_tx);
                }
            }
            Err(error) => give_up("对话框显示失败", &error, cancel_tx),
        }
    })
    .unwrap_or(ContentDialogResult::None)
}

fn give_up(context: &str, error: &windows_core::Error, tx: Sender<ContentDialogResult>) {
    append_crash_log(&format!("{context}: {error}"));
    let _ = tx.send(ContentDialogResult::None);
}

/// 单按钮提示（对偶 `ShowInfoAsync`）。
pub fn info(title: &str, message: &str) {
    let title = title.to_owned();
    let message = message.to_owned();
    show_blocking(move || build(&title, &message, None, None, Some("确定")));
}

/// 二选一确认（对偶 `ShowConfirmAsync`，默认按钮文案「确认 / 取消」）。
pub fn confirm(title: &str, message: &str) -> bool {
    confirm_with(title, message, "确认", "取消")
}

pub fn confirm_with(title: &str, message: &str, primary: &str, close: &str) -> bool {
    let title = title.to_owned();
    let message = message.to_owned();
    let primary = primary.to_owned();
    let close = close.to_owned();

    show_blocking(move || {
        let dialog = build(&title, &message, Some(&primary), None, Some(&close))?;
        dialog.SetDefaultButton(ContentDialogButton::Primary)?;
        Ok(dialog)
    }) == ContentDialogResult::Primary
}

/// 文件名冲突处理（对偶 `ShowConflictResolutionAsync`）。
///
/// 原版用 RadioButtons + 确认/取消，这里把三个选项直接做成对话框的三个按钮，
/// 少一次点击且不需要在 Rust 侧构造 `RadioButton` 项；语义完全一致。
pub fn conflict_resolution(file_name: &str, auto_suffix: Option<i64>) -> Option<ConflictResolutionKind> {
    let file_name = file_name.to_owned();
    let suffix_text = match auto_suffix {
        Some(suffix) => format!("自动加序号后缀 ({suffix})"),
        None => "自动加序号后缀".to_owned(),
    };

    let result = show_blocking(move || {
        build(
            "文件已存在",
            &format!("{file_name} 已存在，请选择处理方式："),
            Some(&suffix_text),
            Some("覆盖现有文件"),
            Some("取消"),
        )
    });

    match result {
        ContentDialogResult::Primary => Some(ConflictResolutionKind::AutoSuffix),
        ContentDialogResult::Secondary => Some(ConflictResolutionKind::Overwrite),
        _ => None,
    }
}

/// 保存用例的用户提问实现：由 `PostSaveUseCase::save` 在工作线程上同步调用。
pub struct UiSavePrompts;

impl SavePrompts for UiSavePrompts {
    fn resolve_conflict(&self, conflict: &ConflictResult) -> Option<ConflictResolutionKind> {
        conflict_resolution(&conflict.file_name(), conflict.auto_suffix())
    }

    fn confirm_overwrite(&self, file_name: &str) -> bool {
        confirm("确认覆盖", &format!("确定覆盖现有文件 {file_name} 吗？"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_kinds_are_distinct() {
        assert_ne!(
            ConflictResolutionKind::AutoSuffix,
            ConflictResolutionKind::Overwrite
        );
    }

    #[test]
    fn prompts_without_dispatcher_answer_cancel() {
        // 单元测试没有 UI 线程：ask_ui 投递失败 → 取消，绝不能阻塞
        assert!(UiSavePrompts
            .resolve_conflict(&ConflictResult::new("a.md".into(), Some(1)))
            .is_none());
        assert!(!UiSavePrompts.confirm_overwrite("a.md"));
    }
}
