//! 入口：STA 套间 → WinAppSDK 运行时 → `Application::Start`。
//!
//! 对偶 .NET 侧由 WinUI 模板生成的启动路径（`Program` 的 BootStrapper + `App.xaml`），
//! 差别有三：
//! 1. 未打包运行，靠 `PackageDependency` 把 WindowsAppRuntime 挂进进程（安装包不带 MSIX）；
//! 2. 显式声明 Per-Monitor V2 DPI 感知——打包模板由 manifest 提供，这里没有 manifest；
//! 3. 没有 `StartupUri`：`App::OnLaunched` 自己建窗口（见 `app.rs`）。
//!
//! 已知风险（详见 `src-rs/README.md`）：上游示例在 WinAppSDK 1.8 上要额外接
//! `ResourceManagerRequested` 并指向 `resources.pri`（microsoft/WindowsAppSDK#5940）。
//! 本项目不产 `.pri`（无 XAML、无资源包），照搬那段只会得到一个必然失败的空资源管理器，
//! 因此不接——若运行期在控件主题里撞到资源查找异常，那是这条差异的表现，不是新 bug。

// cargo 默认把 exe 链成控制台子系统，双击会多开一个黑框；纯 GUI 程序按 windows 子系统走
#![windows_subsystem = "windows"]

mod actions;
mod app;
mod icon;
mod markdown_edit;
mod markdown_view;
mod md_render;
mod pages;
mod runtime;
mod services;
mod view_models;
mod widgets;
mod window;

use windows::Win32::UI::HiDpi::{
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_core::Result;
use winui3::Microsoft::UI::Xaml::{Application, ApplicationInitializationCallback};
use winui3::{XamlApp, bootstrap::PackageDependency};

use crate::app::App;

fn main() {
    if let Err(error) = start() {
        // windows 子系统下没有控制台可读，唯一留痕是崩溃日志
        runtime::append_crash_log(&format!("启动失败：{error}"));
        eprintln!("启动失败：{error}");
        std::process::exit(1);
    }
}

fn start() -> Result<()> {
    // WinUI 3 的 UI 线程必须是 STA：控件树与对话框都调度在这条线程上
    winui3::init_apartment(winui3::ApartmentType::SingleThreaded)?;

    unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };

    // 生命周期与进程一致，句柄只需保活到退出
    let _dependency = PackageDependency::initialize()?;

    // 对偶 `App.OnUnhandledException`：记日志后照常 panic（不在不一致状态下继续跑）
    let _ = std::panic::set_hook(Box::new(|info| {
        runtime::append_crash_log(&info.to_string());
    }));

    Application::Start(&ApplicationInitializationCallback::new(|_| {
        let _app = XamlApp::compose(App)?;

        // 组合根（对偶 `App` 的字段初始化）：先拿到调度队列与服务，
        // 随后框架回调 `OnLaunched`，那时页面与对话框都已可用
        runtime::capture_dispatcher_queue()?;
        let services = services::compose_services(&runtime::app_data_dir());
        actions::project::restore_default_project(&services);

        Ok(())
    }))
}
