//! 博文正文页（对偶 `Pages/PostBodyPage.xaml(.cs)`）。
//!
//! 与 .NET 版的差异全部来自「代码构建 UI」与投影缺件：
//! - `MarkdownTextBlock`（CommunityToolkit）不在 Rust 投影里，实时预览由本项目的
//!   [`crate::md_render`] + [`crate::markdown_view`] 近似渲染：标题/粗斜体/删除线/行内码/
//!   链接/引用/列表/表格/代码块都有对应形态，图片降级为灰色 `[图片]` 占位、脚注与 HTML 按文本走；
//! - `ToggleButton` + `FontIcon` 的预览开关换成 `CheckBox`，工具栏图标全部换成文字标记
//!   （`Segoe Fluent Icons` 的码点能写，但离线无法校验渲染结果，宁可用可读的 ASCII/中文）；
//! - `Style`（`FormatButtonStyle` 等）无法在代码里引用，尺寸逐颗 `MinWidth` + `Padding` 设；
//! - 原版算出 `TargetDirectory` 却没有展示位（FR-4.5~4.9 的落点目录），这里挂在 alt 行下方，
//!   既是提示也让这条派生值不至于变成死代码。
//!
//! 编辑动作全部走 [`crate::markdown_edit`] 的纯函数（输入「文本 + 选区」，输出「新文本 + 新选区」），
//! 页面只负责读控件、写控件、把正文同步进共享编辑态。
//!
//! [`Services::post_form`] 与头信息页同源：`TextChanged` 实时写回，
//! 所以任何动作执行前都不必再手动同步（对偶 .NET 的 `SyncBodyFromTextBox`）。

use std::sync::{Arc, Mutex};

use windows_core::Result as WinResult;
use winui3::Microsoft::UI::Dispatching::DispatcherQueueTimer;
use winui3::Microsoft::UI::Xaml::{
    Controls::{
        Border, Button, CheckBox, ColumnDefinition, Grid, Page, ScrollViewer, StackPanel, TextBlock,
        TextBox,
    },
    VerticalAlignment,
};

use crate::actions::{Refresh, post};
use crate::markdown_edit::{self, Edit, Selection};
use crate::markdown_view;
use crate::md_render;
use crate::runtime::{Debounce, restart_timer};
use crate::services::Services;
use crate::widgets;
use super::{
    Length, PageLifecycle, PostActions, Slot, bind, page_header, post_actions, refresh_of, through,
};

/// 工具栏小按钮的尺寸（对偶 `FormatButtonStyle` 的 `MinWidth=38` / `Padding=8,5`）。
const TOOL_MIN_WIDTH: f64 = 38.0;
const TOOL_PADDING: (f64, f64, f64, f64) = (8.0, 5.0, 8.0, 5.0);
/// 预览防抖，与原版 `_previewTimer` 的 300 ms 一致。
const PREVIEW_DEBOUNCE_MS: u64 = 300;
/// 行距/列距（对偶 `Grid RowSpacing="12"` 与各 `ColumnSpacing="12"`）。
const ROW_SPACING: f64 = 12.0;
const COLUMN_SPACING: f64 = 12.0;

#[derive(Default)]
pub struct PostBodyPage {
    /// 预览防抖的宿主，理由同头信息页：委托不是 `Send`，交给页面结构体保活。
    debounce: Mutex<Option<Debounce>>,
    slot: Slot,
}

crate::xaml_page!(PostBodyPage);

struct Controls {
    root: Grid,
    editor: TextBox,
    preview_column: ColumnDefinition,
    preview_border: Border,
    preview_panel: StackPanel,
    preview_toggle: CheckBox,
    import: Button,
    replace: CheckBox,
    alt: TextBox,
    insert: Button,
    target_directory: TextBlock,
    commands: PostActions,
}

impl PageLifecycle for PostBodyPage {
    fn entered(&self, base: &Page) -> WinResult<()> {
        let services = crate::services::services();
            let controls = Arc::new(build_controls(&services, &through(&self.slot))?);
            let timer = self.start_debounce(&services, &controls)?;
    
        let refresh = refresh_of({
            let services = Arc::clone(&services);
            let controls = Arc::clone(&controls);
            move || push_to_controls(&services, &controls)
        });
        bind(&self.slot, Some(Arc::clone(&refresh)));

        wire_inputs(&services, &controls, &timer, &refresh)?;
    
        base.SetContent(&controls.root)?;
            push_to_controls(&services, &controls);
            Ok(())
    }

    fn departed(&self) {
        bind(&self.slot, None);
        let mut guard = self
            .debounce
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(debounce) = guard.as_mut() {
            let _ = debounce.stop();
        }
        *guard = None;
    }
}

impl PostBodyPage {
    /// 建防抖：到期只把正文推给预览块（不重建控件树）。
    fn start_debounce(&self, services: &Arc<Services>, controls: &Arc<Controls>) -> WinResult<DispatcherQueueTimer> {
        let tick_services = Arc::clone(services);
        let tick_controls = Arc::clone(controls);
        let debounce = Debounce::new(PREVIEW_DEBOUNCE_MS, move || {
            push_preview(&tick_services, &tick_controls);
        })?;

        let handle = debounce.handle();
        *self
            .debounce
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(debounce);
        Ok(handle)
    }
}

// ---------------------------------------------------------------- 控件树

fn build_controls(services: &Arc<Services>, actions_refresh: &Refresh) -> WinResult<Controls> {
    let form = services.form();
    let project_selected = services.project.current().is_some();

    let commands = post_actions(Arc::clone(services), Arc::clone(actions_refresh))?;
    let header = page_header("博文正文", Some(&commands.root))?;

    // 行 1：导入正文 + 「替换而非追加」
    let import = widgets::button("导入正文...")?;
    let replace = widgets::check_box("替换而非追加", form.replace_body_on_import)?;
    replace.SetVerticalAlignment(VerticalAlignment::Center)?;
    let import_row = widgets::hstack(COLUMN_SPACING)?;
    widgets::add(&import_row, &import)?;
    widgets::add(&import_row, &replace)?;

    // 行 2：alt 文本 + 插入图片
    let alt_label = widgets::muted_text("alt 文本")?;
    alt_label.SetVerticalAlignment(VerticalAlignment::Center)?;
    let alt = widgets::text_box("", &form.alt_text)?;
    widgets::placeholder(&alt, "留空则生成 ![](...)；图片将插入到正文光标处")?;
    let insert = widgets::button("插入图片...")?;
    widgets::enabled(&insert, form.can_insert_images(project_selected))?;
    let target_directory = widgets::muted_text(&form.target_directory(project_selected))?;

    let image_row = widgets::grid(&[], &[Length::Auto, Length::Star(1.0), Length::Auto])?;
    image_row.SetColumnSpacing(COLUMN_SPACING)?;
    widgets::place(&alt_label, 0, 0)?;
    widgets::put(&image_row, &alt_label)?;
    widgets::place(&alt, 0, 1)?;
    widgets::put(&image_row, &alt)?;
    widgets::place(&insert, 0, 2)?;
    widgets::put(&image_row, &insert)?;

    let image_panel = widgets::vstack(4.0)?;
    widgets::add(&image_panel, &image_row)?;
    widgets::add(&image_panel, &target_directory)?;

    // 行 3：Markdown 工具栏（横向可滚） + 预览开关
    let editor = widgets::body_editor("在此编辑博文正文，或使用上方工具导入/插入内容...")?;
    let tools = build_toolbar(&editor)?;
    let tools_viewer = widgets::h_scroll_viewer(&tools)?;
    let preview_toggle = widgets::check_box("预览", true)?;
    preview_toggle.SetVerticalAlignment(VerticalAlignment::Center)?;

    let toolbar_row = widgets::grid(&[], &[Length::Star(1.0), Length::Auto])?;
    toolbar_row.SetColumnSpacing(COLUMN_SPACING)?;
    widgets::place(&tools_viewer, 0, 0)?;
    widgets::put(&toolbar_row, &tools_viewer)?;
    widgets::place(&preview_toggle, 0, 1)?;
    widgets::put(&toolbar_row, &preview_toggle)?;

    // 行 4：编辑器 + 实时预览（预览面板内容由 push_preview 重建）
    let preview_panel = widgets::vstack(8.0)?;
    let preview_border = bordered(&widgets::scroll_viewer(&preview_panel)?)?;
    let editor_grid = widgets::grid(&[], &[])?;
    editor_grid.SetColumnSpacing(COLUMN_SPACING)?;
    widgets::append_column(&editor_grid, &Length::Star(1.0))?;
    let preview_column = widgets::append_column(&editor_grid, &Length::Star(1.0))?;
    widgets::place(&editor, 0, 0)?;
    widgets::put(&editor_grid, &editor)?;
    widgets::place(&preview_border, 0, 1)?;
    widgets::put(&editor_grid, &preview_border)?;

    let root = widgets::grid(
        &[
            Length::Auto,
            Length::Auto,
            Length::Auto,
            Length::Auto,
            Length::Star(1.0),
        ],
        &[],
    )?;
    root.SetRowSpacing(ROW_SPACING)?;
    // 五段里 Grid 与 StackPanel 混排，逐段挂行。
    widgets::place(&header.root, 0, 0)?;
    widgets::put(&root, &header.root)?;
    widgets::place(&import_row, 1, 0)?;
    widgets::put(&root, &import_row)?;
    widgets::place(&image_panel, 2, 0)?;
    widgets::put(&root, &image_panel)?;
    widgets::place(&toolbar_row, 3, 0)?;
    widgets::put(&root, &toolbar_row)?;
    widgets::place(&editor_grid, 4, 0)?;
    widgets::put(&root, &editor_grid)?;

    Ok(Controls {
        root,
        editor,
        preview_column,
        preview_border,
        preview_panel,
        preview_toggle,
        import,
        replace,
        alt,
        insert,
        target_directory,
        commands,
    })
}

/// 预览外框：与卡片同款描边，内边距对齐原版 `Padding="12,10"`。
fn bordered(content: &ScrollViewer) -> WinResult<Border> {
    let border = Border::new()?;
    border.SetPadding(widgets::insets(12.0, 10.0, 12.0, 10.0))?;
    border.SetCornerRadius(widgets::uniform_radius(8.0))?;
    border.SetBackground(&widgets::card_fill()?)?;
    border.SetBorderThickness(widgets::thickness(1.0))?;
    border.SetBorderBrush(&widgets::card_stroke()?)?;
    border.SetChild(content)?;
    Ok(border)
}

/// 工具栏：包裹类 → 行首类 → 代码/链接 → 列表/表格，分组之间插一根分隔线。
fn build_toolbar(editor: &TextBox) -> WinResult<StackPanel> {
    let panel = widgets::hstack(4.0)?;

    add_tool(&panel, "B", editor, |selection| {
        markdown_edit::wrap("**", selection)
    })?;
    add_tool(&panel, "I", editor, |selection| {
        markdown_edit::wrap("*", selection)
    })?;
    add_tool(&panel, "~~", editor, |selection| {
        markdown_edit::wrap("~~", selection)
    })?;
    widgets::add(&panel, &divider()?)?;

    add_tool(&panel, "H2", editor, |selection| {
        markdown_edit::line_prefix("## ", selection)
    })?;
    add_tool(&panel, "H3", editor, |selection| {
        markdown_edit::line_prefix("### ", selection)
    })?;
    add_tool(&panel, "“", editor, |selection| {
        markdown_edit::line_prefix("> ", selection)
    })?;
    widgets::add(&panel, &divider()?)?;

    add_tool(&panel, "</>", editor, |selection| {
        markdown_edit::wrap("`", selection)
    })?;
    add_tool(&panel, "代码块", editor, markdown_edit::code_block)?;
    add_tool(&panel, "链接", editor, markdown_edit::link)?;
    widgets::add(&panel, &divider()?)?;

    add_tool(&panel, "≡", editor, |selection| {
        markdown_edit::line_prefix("- ", selection)
    })?;
    add_tool(&panel, "1.", editor, |selection| {
        markdown_edit::line_prefix("1. ", selection)
    })?;
    add_tool(&panel, "表格", editor, markdown_edit::table)?;

    Ok(panel)
}

fn add_tool<F>(panel: &StackPanel, label: &str, editor: &TextBox, apply: F) -> WinResult<()>
where
    F: Fn(&Selection) -> Edit + Send + 'static,
{
    let button = widgets::button(label)?;
    widgets::min_width(&button, TOOL_MIN_WIDTH)?;
    widgets::padding(&button, widgets::insets(TOOL_PADDING.0, TOOL_PADDING.1, TOOL_PADDING.2, TOOL_PADDING.3))?;

    let editor = editor.clone();
    widgets::on_click(&button, move || {
        let _ = apply_edit(&editor, &apply);
    })?;
    widgets::add(panel, &button)
}

/// 工具栏分隔线（对偶 `ToolbarDividerStyle`：1px 宽、上下留 6）。
fn divider() -> WinResult<Border> {
    let line = Border::new()?;
    line.SetWidth(1.0)?;
    line.SetBackground(&widgets::card_stroke()?)?;
    widgets::margin(&line, widgets::insets(4.0, 6.0, 4.0, 6.0))?;
    Ok(line)
}

/// 读控件 → 纯函数编辑 → 写回控件 → 光标还给编辑器。
///
/// 写回会触发一次 `TextChanged`，正文因此自动同步进共享编辑态
/// （对偶原版每次编辑后调用的 `SyncBodyFromTextBox`）。
fn apply_edit<F>(editor: &TextBox, apply: F) -> WinResult<()>
where
    F: Fn(&Selection) -> Edit,
{
    let selection = widgets::snapshot(editor)?;
    let edit = apply(&selection);
    widgets::apply_edit(editor, &edit)?;
    widgets::focus(editor)
}

// ---------------------------------------------------------------- 事件接线

fn wire_inputs(
    services: &Arc<Services>,
    controls: &Arc<Controls>,
    timer: &DispatcherQueueTimer,
    refresh: &Refresh,
) -> WinResult<()> {
    // 正文：实时写回 + 重启预览防抖
    let editor = controls.editor.clone();
    let pull_services = Arc::clone(services);
    let pull_timer = timer.clone();
    widgets::on_text_changed(&controls.editor, move || {
        let body = widgets::value_of(&editor);
        pull_services.edit_form(|form| form.set_body(body));
        let _ = restart_timer(&pull_timer);
    })?;

    // alt 文本与「替换而非追加」都不影响预览，写完即可
    let alt = controls.alt.clone();
    let alt_services = Arc::clone(services);
    widgets::on_text_changed(&controls.alt, move || {
        let value = widgets::value_of(&alt);
        alt_services.edit_form(|form| form.alt_text = value);
    })?;

    let replace_services = Arc::clone(services);
    widgets::on_check_changed(&controls.replace, move |checked| {
        replace_services.edit_form(|form| form.replace_body_on_import = checked);
    })?;

    // 导入正文：原版固定追加到末尾（`InsertAtCursor(Body, imported, null)`），不传光标
    widgets::on_click(&controls.import, {
        let services = Arc::clone(services);
        let refresh = Arc::clone(refresh);
        move || post::import_body(Arc::clone(&services), None, Arc::clone(&refresh))
    })?;

    let editor = controls.editor.clone();
    widgets::on_click(&controls.insert, {
        let services = Arc::clone(services);
        let refresh = Arc::clone(refresh);
        move || {
            let caret = widgets::caret_of(&editor);
            post::insert_images(Arc::clone(&services), caret, Arc::clone(&refresh))
        }
    })?;

    // 预览开关：收起时把列宽压成 0（对偶 `OnPreviewToggleClick` 的两行赋值）
    let border = controls.preview_border.clone();
    let column = controls.preview_column.clone();
    widgets::on_check_changed(&controls.preview_toggle, move |show| {
        let _ = widgets::visible(&border, show);
        let _ = widgets::set_column_width(
            &column,
            &if show {
                Length::Star(1.0)
            } else {
                Length::Pixel(0.0)
            },
        );
    })
}

// ---------------------------------------------------------------- 状态 → 控件

fn push_to_controls(services: &Services, controls: &Controls) {
    let form = services.form();
    let project_selected = services.project.current().is_some();

    let _ = widgets::set_value(&controls.alt, &form.alt_text);
    let _ = widgets::set_checked(&controls.replace, form.replace_body_on_import);
    let _ = widgets::enabled(&controls.insert, form.can_insert_images(project_selected));
    let _ = widgets::set_block_text(&controls.target_directory, &form.target_directory(project_selected));
    let _ = controls.commands.set_busy(form.is_busy);

    // 正文是这一页的主体：直接推，不等防抖（防抖只服务连续按键）
    let _ = widgets::set_value(&controls.editor, &form.body);
    push_preview(services, controls);
}

fn push_preview(services: &Services, controls: &Controls) {
    let form = services.form();
    let panel = &controls.preview_panel;
    let _ = widgets::clear_children(panel);
    if form.body.trim().is_empty() {
        if let Ok(hint) = widgets::muted_text("正文为空：编辑后这里显示渲染后的 Markdown 预览") {
            let _ = widgets::add(panel, &hint);
        }
        return;
    }
    let _ = markdown_view::render(&md_render::parse(&form.body), panel);
}
