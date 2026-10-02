//! 项目页（对偶 `Pages/ProjectPage.xaml(.cs)`）。
//!
//! 原版是缓存页（`NavigationCacheMode="Required"`）：VM 在构造函数里建、`OnLoaded` 里
//! `RefreshFromContext()`。这里每次导航重建控件树并当场读一次当前项目，
//! 效果与「缓存页 + 每次 Loaded 回填」相同，却不必维护页面级状态。

use std::sync::Arc;

use windows_core::Result as WinResult;
use winui3::Microsoft::UI::Xaml::Controls::{Button, Page, StackPanel};

use crate::actions::project::{self, ExternalApp};
use crate::actions::Refresh;
use crate::services::Services;
use crate::view_models::PROJECT_PATH_PLACEHOLDER;
use crate::widgets;
use super::{PageLifecycle, page_header, page_root, refresh_of};

#[derive(Default)]
pub struct ProjectPage;

crate::xaml_page!(ProjectPage);

impl PageLifecycle for ProjectPage {
    fn entered(&self, base: &Page) -> WinResult<()> {
        let services = crate::services::services();
        let header = page_header("项目", None)?;

        // 路径行是唯一会被动作改写的显示值：换项目后只刷新它
        let path_text = widgets::preview_text(&display_path(&services))?;
        let refresh = refresh_of({
            let shown = path_text.clone();
            let services = Arc::clone(&services);
            move || {
                let _ = widgets::set_block_text(&shown, &display_path(&services));
            }
        });

        let content = widgets::vstack(4.0)?;
        widgets::add(&content, &widgets::body_text("项目路径")?)?;
        widgets::add(&content, &path_text)?;
        widgets::add(&content, &action_row(&services, &refresh)?)?;

        let body = widgets::vstack(12.0)?;
        widgets::add(&body, &widgets::card(&content)?)?;

        base.SetContent(&page_root(header, &body)?)
    }
}

/// 一颗按钮 + 它要执行的命令。三颗按钮只差这一个闭包，用循环写反而更绕。
fn command_button(
    label: &str,
    services: &Arc<Services>,
    refresh: &Refresh,
    run: impl Fn(Arc<Services>, Refresh) + Send + 'static,
) -> WinResult<Button> {
    let button = widgets::button(label)?;
    let services = Arc::clone(services);
    let refresh = Arc::clone(refresh);
    widgets::on_click(&button, move || run(Arc::clone(&services), Arc::clone(&refresh)))?;
    Ok(button)
}

fn action_row(services: &Arc<Services>, refresh: &Refresh) -> WinResult<StackPanel> {
    let row = widgets::hstack(8.0)?;

    let select = command_button("选择项目路径", services, refresh, |services, refresh| {
        project::select_folder(services, refresh);
    })?;
    let explorer = command_button("资源管理器", services, refresh, |services, refresh| {
        project::open_externally(services, ExternalApp::Explorer, refresh);
    })?;
    let vscode = command_button("VS Code", services, refresh, |services, refresh| {
        project::open_externally(services, ExternalApp::VsCode, refresh);
    })?;

    widgets::add(&row, &select)?;
    widgets::add(&row, &explorer)?;
    widgets::add(&row, &vscode)?;
    Ok(row)
}

/// 未选择项目时给出占位文案（对偶 `ProjectPageViewModel.ProjectPath` 初值）。
fn display_path(services: &Services) -> String {
    services
        .project
        .path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| PROJECT_PATH_PLACEHOLDER.to_owned())
}
