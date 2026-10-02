//! 项目命令（对偶 `ProjectPageViewModel` 的三个 RelayCommand）。

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::Arc;

use jp_domain::projects::BlogProject;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::Refresh;
use crate::runtime::spawn_work;
use crate::services::{dialogs, pickers, Services};

/// 恢复上次使用的项目。
///
/// 启动期唯一一次在 UI 线程上的同步读盘：只读一个几百字节的 json，
/// 换来「窗口出现即可用」，省掉一次异步回填与首屏空态。
/// 读失败或目录已不存在都按「无默认项目」处理，不影响启动（对偶 .NET 侧的空 catch）。
pub fn restore_default_project(services: &Services) {
    let Ok(Some(path)) = services.settings.get() else {
        return;
    };
    let project = BlogProject::new(&path);
    if project.path().is_dir() {
        services.project.set(Some(project));
    }
}

/// 选择项目目录并记为默认项目（对偶 `SelectProjectPathAsync`）。
pub fn select_folder(services: Arc<Services>, refresh: Refresh) {
    spawn_work(
        move || {
            let Some(path) = pickers::pick_folder() else {
                return;
            };
            if !path.is_dir() {
                dialogs::info("路径无效", "所选文件夹不存在。");
                return;
            }

            let display = path.display().to_string();
            services.project.set(Some(BlogProject::new(path)));

            // 记住默认项目失败只影响下次启动，不打断本次操作
            if let Err(error) = services.settings.set(Some(&display)) {
                dialogs::info("保存默认项目失败", &error.to_string());
            }
        },
        move |_: ()| super::notify(&refresh),
    )
}

/// 用资源管理器 / VS Code 打开项目根目录（对偶 `OpenInExplorerAsync` / `OpenInVsCodeAsync`）。
pub fn open_externally(services: Arc<Services>, target: ExternalApp, refresh: Refresh) {
    spawn_work(
        move || {
            let Some(project) = services.project.current() else {
                dialogs::info("未选择项目", "请先选择一个博客项目。");
                return;
            };

            let path = project.path().to_string_lossy().into_owned();
            let (program, hint) = match target {
                ExternalApp::Explorer => ("explorer.exe", String::new()),
                ExternalApp::VsCode => (
                    "code",
                    "请确认 VS Code 已安装且 `code` 命令在 PATH 中。".to_owned(),
                ),
            };

            match launch(program, &path) {
                Ok(()) => {}
                Err(error) => {
                    let message = if hint.is_empty() {
                        format!("无法在资源管理器中打开项目：{error}")
                    } else {
                        format!("无法在 VS Code 中打开项目。{hint}\r\n错误：{error}")
                    };
                    dialogs::info("打开失败", &message);
                }
            }
        },
        move |_: ()| super::notify(&refresh),
    )
}

/// 经 Shell 启动外部程序：与 .NET `UseShellExecute=true` 对偶。
///
/// 必须走 `ShellExecuteW` 而不是 `Command::spawn`（CreateProcess）——VS Code 在 PATH 上
/// 是 `code.cmd` 批处理，CreateProcess 只认 .exe，症状就是「VS Code」按钮点了没反应。
/// 返回 `HINSTANCE`，≤32 为 SE_ERR_*（数值与 Win32 错误码同源，直接喂给 io::Error）。
fn launch(program: &str, path: &str) -> std::io::Result<()> {
    let operation = wide("open");
    let file = wide(program);
    let parameters = wide(path);
    let directory = wide("");

    // SAFETY: 四个串均为 NUL 结尾宽字符的栈上缓冲，ShellExecuteW 只读不存
    let instance = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR(parameters.as_ptr()),
            PCWSTR(directory.as_ptr()),
            SW_SHOWNORMAL,
        )
    };

    let code = instance.0 as isize;
    if code <= 32 {
        return Err(std::io::Error::from_raw_os_error(code as i32));
    }
    Ok(())
}

fn wide(text: &str) -> Vec<u16> {
    OsStr::new(text).encode_wide().chain(std::iter::once(0)).collect()
}

/// 外部编辑器/文件管理器。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalApp {
    Explorer,
    VsCode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_app_variants_are_distinct() {
        assert_ne!(ExternalApp::Explorer, ExternalApp::VsCode);
    }

    #[test]
    fn launching_a_missing_tool_reports_error() {
        assert!(launch("definitely-not-a-command", "C:\\").is_err());
    }
}
