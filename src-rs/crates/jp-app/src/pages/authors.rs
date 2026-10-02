//! 作者管理页（对偶 `Pages/AuthorsPage.xaml(.cs)`）。
//!
//! 原版不是缓存页，每次导航 `new AuthorsPageViewModel()`，所以页面状态（列表 + 草稿）
//! 也在 `entered()` 里新建一份。
//!
//! 一处投影替换：`ListView` 的 `ItemTemplate`（id 粗体 + 名称次要色两行）在代码里给不出
//! `DataTemplate`，退化成单行 `"{id} ({name})"`；其余排布照抄。

use std::sync::{Arc, Mutex};

use windows_core::Result as WinResult;
use winui3::Microsoft::UI::Xaml::Controls::{Button, Grid, ListView, Page, TextBox};

use crate::actions::authors;
use crate::actions::{PageState, Refresh, held, notify};
use crate::runtime::append_crash_log;
use crate::view_models::{self, AuthorsState};
use crate::widgets;
use super::{Length, PageLifecycle, page_header, refresh_of, state};

/// 左栏宽度（对偶 `ColumnDefinitions="280,*"`），卡片左右内边距各 16。
const LIST_COLUMN_WIDTH: f64 = 280.0;
const CARD_PADDING: f64 = 16.0;

#[derive(Default)]
pub struct AuthorsPage;

crate::xaml_page!(AuthorsPage);

/// 会被状态刷新的控件集合。`Arc` 包裹是为了让 `Refresh` 闭包捕获它。
struct Controls {
    list: ListView,
    id: TextBox,
    name: TextBox,
    twitter: TextBox,
    url: TextBox,
    delete: Button,
    /// 已渲染的列表文本；与目标不一致才重建（重建必然清掉选中项）。
    rendered: Mutex<Vec<String>>,
}

impl PageLifecycle for AuthorsPage {
    fn entered(&self, base: &Page) -> WinResult<()> {
        let services = crate::services::services();
        let model: PageState<view_models::AuthorsPage> = state(view_models::AuthorsPage::default());
        let controls = Arc::new(Controls {
            list: widgets::list_view()?,
            id: widgets::text_box("id *", "")?,
            name: widgets::text_box("name *", "")?,
            twitter: widgets::text_box("twitter", "")?,
            url: widgets::text_box("url", "")?,
            delete: widgets::button("删除")?,
            rendered: Mutex::new(Vec::new()),
        });

        let refresh = refresh_of({
            let controls = Arc::clone(&controls);
            let model = Arc::clone(&model);
            move || push_to_controls(&controls, &model)
        });

        wire_inputs(&controls, &model)?;
        wire_list(&controls, &model, &refresh)?;

        let add = command("新增作者", {
            let (model, refresh) = (Arc::clone(&model), Arc::clone(&refresh));
            move || authors::begin_new(Arc::clone(&model), Arc::clone(&refresh))
        })?;
        let save = command("保存", {
            let (services, model, refresh) =
                (Arc::clone(&services), Arc::clone(&model), Arc::clone(&refresh));
            move || authors::save(Arc::clone(&services), Arc::clone(&model), Arc::clone(&refresh))
        })?;
        let cancel = command("取消", {
            let (model, refresh) = (Arc::clone(&model), Arc::clone(&refresh));
            move || authors::cancel(Arc::clone(&model), Arc::clone(&refresh))
        })?;
        widgets::on_click(&controls.delete, {
            let (services, model, refresh) =
                (Arc::clone(&services), Arc::clone(&model), Arc::clone(&refresh));
            move || authors::delete(Arc::clone(&services), Arc::clone(&model), Arc::clone(&refresh))
        })?;

        let header_actions = widgets::hstack(8.0)?;
        widgets::add(&header_actions, &add)?;
        let header = page_header("作者", Some(&header_actions))?;

        let body = content_grid(&controls, &save, &cancel)?;
        let root = page_grid(&header, &body)?;
        base.SetContent(&root)?;

        // 对偶 `OnLoaded → LoadAuthorsCommand.Execute(null)`
        authors::load(Arc::clone(&services), Arc::clone(&model), Arc::clone(&refresh));
        Ok(())
    }
}

/// 状态 → 控件；每次动作收尾都会经 `notify` 投回 UI 线程执行一遍。
fn push_to_controls(controls: &Arc<Controls>, model: &PageState<view_models::AuthorsPage>) {
    let snapshot = held(model).clone();

    let texts: Vec<String> = snapshot
        .authors
        .iter()
        .map(|author| format!("{} ({})", author.id(), author.name()))
        .collect();
    if rendered(controls) != texts {
        match widgets::fill_list(&controls.list, &texts) {
            Ok(()) => *controls.rendered.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) =
                texts,
            // 刷新在 UI 线程跑，绝不能在这里弹对话框（那会自锁），只落崩溃日志
            Err(error) => append_crash_log(&format!("重建作者列表失败：{error}")),
        }
    }

    let selected = snapshot
        .authors
        .iter()
        .position(|author| Some(author.id()) == snapshot.draft.selected_id.as_deref());

    let draft = &snapshot.draft;
    let _ = widgets::set_list_index(&controls.list, selected);
    let _ = widgets::set_value(&controls.id, &draft.id);
    let _ = widgets::set_value(&controls.name, &draft.name);
    let _ = widgets::set_value(&controls.twitter, &draft.twitter);
    let _ = widgets::set_value(&controls.url, &draft.url);

    // 对偶 `IsReadOnly`：id 输入框只读 + 删除可用，同一判定
    let locked = draft.id_locked();
    let _ = widgets::read_only(&controls.id, locked);
    let _ = widgets::enabled(&controls.delete, locked);
}

fn rendered(controls: &Controls) -> Vec<String> {
    controls
        .rendered
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// 四个输入框实时写回草稿（对偶 `UpdateSourceTrigger=PropertyChanged`）。
fn wire_inputs(
    controls: &Arc<Controls>,
    model: &PageState<view_models::AuthorsPage>,
) -> WinResult<()> {
    pull(&controls.id, model, |draft, value| draft.id = value)?;
    pull(&controls.name, model, |draft, value| draft.name = value)?;
    pull(&controls.twitter, model, |draft, value| draft.twitter = value)?;
    pull(&controls.url, model, |draft, value| draft.url = value)
}

fn pull<F>(
    target: &TextBox,
    model: &PageState<view_models::AuthorsPage>,
    write: F,
) -> WinResult<()>
where
    F: Fn(&mut AuthorsState, String) + Send + 'static,
{
    let control = target.clone();
    let model = Arc::clone(model);
    widgets::on_text_changed(target, move || {
        let value = widgets::value_of(&control);
        write(&mut held(&model).draft, value);
    })
}

/// 列表选中 → 草稿回填（对偶 `OnSelectedAuthorChanged`）。
///
/// 程序化 `SetSelectedIndex` 同样触发这里，但 `apply_selection` 幂等：
/// 同一行第二次应用结果不变。清空选中（下标 `-1`）按 .NET 语义不动草稿。
fn wire_list(
    controls: &Arc<Controls>,
    model: &PageState<view_models::AuthorsPage>,
    refresh: &Refresh,
) -> WinResult<()> {
    let model = Arc::clone(model);
    let refresh = Arc::clone(refresh);
    widgets::on_list_changed(&controls.list, move |index| {
        let author = index.and_then(|index| held(&model).authors.get(index).cloned());
        if author.is_some() {
            authors::apply_selection(&model, author.as_ref());
            notify(&refresh);
        }
    })
}

/// UI 线程命令按钮：`on_click` 的闭包要求 `'static`，所以每条命令自己克隆句柄。
fn command(label: &str, run: impl Fn() + Send + 'static) -> WinResult<Button> {
    let button = widgets::button(label)?;
    widgets::on_click(&button, run)?;
    Ok(button)
}

/// 主体：左列表卡 + 右表单卡（对偶 `ColumnDefinitions="280,*"`，间距 24）。
fn content_grid(controls: &Controls, save: &Button, cancel: &Button) -> WinResult<Grid> {
    let left = widgets::vstack(12.0)?;
    widgets::add(&left, &widgets::section_text("作者列表")?)?;
    widgets::min_width(&controls.list, LIST_COLUMN_WIDTH - 2.0 * CARD_PADDING)?;
    widgets::add(&left, &controls.list)?;
    let left_card = widgets::card(&left)?;

    let fields = widgets::vstack(16.0)?;
    for field in [&controls.id, &controls.name, &controls.twitter, &controls.url] {
        widgets::add(&fields, field)?;
    }
    let buttons = widgets::hstack(8.0)?;
    widgets::add(&buttons, save)?;
    widgets::add(&buttons, cancel)?;
    widgets::add(&buttons, &controls.delete)?;
    widgets::add(&fields, &buttons)?;

    let right = widgets::vstack(12.0)?;
    widgets::add(&right, &widgets::section_text("作者信息")?)?;
    widgets::add(&right, &fields)?;
    let right_card = widgets::card(&right)?;

    let grid = widgets::grid(&[], &[])?;
    grid.SetColumnSpacing(24.0)?;
    widgets::append_column(&grid, &Length::Pixel(LIST_COLUMN_WIDTH))?;
    widgets::append_column(&grid, &Length::Star(1.0))?;
    widgets::place(&left_card, 0, 0)?;
    widgets::put(&grid, &left_card)?;
    widgets::place(&right_card, 0, 1)?;
    widgets::put(&grid, &right_card)?;

    Ok(grid)
}

/// 页头 + 主体两行（对偶外层 `Grid RowSpacing="24"`，行 `[Auto,*]`）。
/// 原版主体不套 `ScrollViewer`：两栏各自撑满，列表自己滚。
fn page_grid(header: &super::PageHeader, body: &Grid) -> WinResult<Grid> {
    let root = widgets::grid(&[Length::Auto, Length::Star(1.0)], &[])?;
    root.SetRowSpacing(24.0)?;
    widgets::place(&header.root, 0, 0)?;
    widgets::put(&root, &header.root)?;
    widgets::place(body, 1, 0)?;
    widgets::put(&root, body)?;
    Ok(root)
}
