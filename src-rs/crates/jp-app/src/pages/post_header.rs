//! 博文头信息页（对偶 `Pages/PostPage.xaml(.cs)`）。
//!
//! 与 .NET 版的结构差异全部来自「代码构建 UI」：
//! - `CalendarDatePicker` / `TimePicker` 在投影里能建但样式与本地化都要自己撑，
//!   改成两个文本框（`YYYY-MM-DD` / `HH:mm`），解析规则在 `PostForm` 里；
//! - `DropDownButton` + `Flyout` 里的作者多选，改成卡片内平铺的 `CheckBox` 列表，
//!   下拉按钮上的 `SelectedAuthorsDisplay` 退化为列表下方的一行摘要；
//! - `CommunityToolkit` 的 `SettingsCard` 换成 [`super::section`]（圆角卡片 + 小标题）；
//! - 宽窄布局仍按 **逻辑像素** 判定（`SizeChanged` + `WIDE_LAYOUT_MIN_WIDTH`），
//!   改写的内容与原版 `NarrowLayout` 的三个 setter 一一对应。
//!
//! 状态是 [`Services::post_form`] 这一份共享编辑态（对偶 `App.Current.PostPageViewModel` 单例），
//! 与正文页之间不传参、直接共用。
//!
//! [`Services::post_form`]: crate::services::Services

use std::sync::{Arc, Mutex};

use jp_domain::authors::Author;
use windows_core::{Interface, Param, Result as WinResult};
use winui3::Microsoft::UI::Dispatching::DispatcherQueueTimer;
use winui3::Microsoft::UI::Xaml::Controls::{
    Border, Button, CheckBox, ComboBox, ColumnDefinition, Grid, Page, StackPanel, TextBlock, TextBox,
};
use winui3::Microsoft::UI::Xaml::{FrameworkElement, UIElement};

use crate::actions::{PageState, Refresh, authors, held, post};
use crate::runtime::{Debounce, restart_timer};
use crate::services::Services;
use crate::view_models::{AuthorOption, PostForm};
use crate::widgets;
use super::{
    Length, PageHeader, PageLifecycle, PostActions, Slot, WIDE_LAYOUT_MIN_WIDTH, bind,
    on_size_changed, page_header, post_actions, refresh_of, section, state, through,
};

/// 日期/时间/时区三格的最小宽度（对偶 `MinWidth="168"` / `"108"` / `ComboBox MinWidth="84"`）。
const DATE_WIDTH: f64 = 168.0;
const TIME_WIDTH: f64 = 108.0;
const ZONE_WIDTH: f64 = 84.0;
/// 预览防抖，与原版 `_previewTimer` 的 150 ms 一致。
const PREVIEW_DEBOUNCE_MS: u64 = 150;
/// 分栏比例（对偶 `Width="3*"` 与 `Width="2*"`）。
const FORM_WEIGHT: f64 = 3.0;
const PREVIEW_WEIGHT: f64 = 2.0;
/// 分栏时列间距，窄窗时归零（对偶 `ColumnSpacing="24"` / setter `0`）。
const COLUMN_SPACING: f64 = 24.0;
/// 卡片之间与外层行距（对偶 `StackPanel Spacing="16"` / `Grid RowSpacing="16"`）。
const CARD_GAP: f64 = 16.0;
/// 卡片内字段间距（对偶 `StackPanel Spacing="12"`）。
const FIELD_SPACING: f64 = 12.0;

#[derive(Default)]
pub struct PostHeaderPage {
    /// 防抖计时器的宿主。`Debounce` 内含事件委托、不满足 `Send`，
    /// 而页面结构体由原生对象持引用，正好放这儿；事件回调只拿 [`Debounce::handle`]。
    debounce: Mutex<Option<Debounce>>,
    /// 刷新闭包的插槽：命令条要在刷新闭包之前建好，见 [`through`]。
    slot: Slot,
}

crate::xaml_page!(PostHeaderPage);

/// 会被状态刷新的控件，外加宽窄布局要改写的三个对象。
struct Controls {
    header: PageHeader,
    content: Grid,
    preview_column: ColumnDefinition,
    preview_panel: StackPanel,
    title: TextBox,
    date: TextBox,
    time: TextBox,
    zone: ComboBox,
    category1: TextBox,
    category2: TextBox,
    tags: TextBox,
    description: TextBox,
    extract: Button,
    authors: StackPanel,
    authors_summary: TextBlock,
    author_boxes: Arc<Mutex<Vec<(String, CheckBox)>>>,
    /// 已渲染的作者候选 id；不一致才重建面板（重建会丢焦点，不能每刷一次就来一遍）。
    rendered_authors: Mutex<Vec<String>>,
    file_name: TextBlock,
    front_matter: TextBlock,
    commands: PostActions,
}

impl PageLifecycle for PostHeaderPage {
    fn entered(&self, base: &Page) -> WinResult<()> {
        let services = crate::services::services();
        let candidates: PageState<Vec<Author>> = state(Vec::new());
        let controls = Arc::new(build_controls(&services, &through(&self.slot))?);

        let timer = self.start_debounce(&services, &controls)?;

        let refresh = refresh_of({
            let services = Arc::clone(&services);
            let controls = Arc::clone(&controls);
            let candidates = Arc::clone(&candidates);
            let timer = timer.clone();
            move || push_to_controls(&services, &controls, &candidates, &timer)
        });
        bind(&self.slot, Some(Arc::clone(&refresh)));

        wire_inputs(&services, &controls, &timer)?;
        widgets::on_click(&controls.extract, {
            let services = Arc::clone(&services);
            let refresh = Arc::clone(&refresh);
            move || post::extract_keywords(Arc::clone(&services), Arc::clone(&refresh))
        })?;

        let root = assemble(&controls)?;
        observe_width(base, &controls)?;
        base.SetContent(&root)?;

        push_to_controls(&services, &controls, &candidates, &timer);
        // 对偶 `OnLoaded → LoadAuthorsCommand.Execute(null)`
        authors::available(Arc::clone(&services), candidates, refresh);
        Ok(())
    }

    fn departed(&self) {
        // 解绑 + 停表：离开页面后工作线程的迟到通知不该再改写已摘除的控件树，
        // 顺带断开 `Controls → 控件 → 插槽 → Refresh → Controls` 的引用环
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

impl PostHeaderPage {
    /// 建防抖：到期只做「重算预览 + 推回两块预览文本」。
    fn start_debounce(
        &self,
        services: &Arc<Services>,
        controls: &Arc<Controls>,
    ) -> WinResult<DispatcherQueueTimer> {
        let tick_services = Arc::clone(services);
        let tick_controls = Arc::clone(controls);
        let debounce = Debounce::new(PREVIEW_DEBOUNCE_MS, move || {
            tick_services.edit_form(PostForm::update_preview);
            push_preview(&tick_controls);
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

    let title = widgets::text_box("title *", &form.state.title)?;

    let date = widgets::text_box("", "")?;
    widgets::placeholder(&date, "选择日期（YYYY-MM-DD）")?;
    widgets::min_width(&date, DATE_WIDTH)?;

    let time = widgets::text_box("", "")?;
    widgets::placeholder(&time, "HH:mm")?;
    widgets::min_width(&time, TIME_WIDTH)?;

    let zone = widgets::combo_box("", &form.time_zone_options(), form.time_zone_index())?;
    widgets::min_width(&zone, ZONE_WIDTH)?;

    let category1 = widgets::text_box("主分类", &form.state.category1)?;
    let category2 = widgets::text_box("子分类", &form.state.category2)?;
    widgets::enabled(&category2, form.has_category1())?;

    let tags = widgets::text_box("", &form.state.tags)?;
    widgets::placeholder(&tags, "用空格分隔多个标签")?;
    let extract = widgets::button("AI 提取关键字")?;

    let authors_panel = widgets::vstack(4.0)?;
    let authors_summary = widgets::muted_text(&form.authors_display())?;
    let description = widgets::description_box("", &form.state.description)?;

    let file_name = widgets::preview_text(&form.file_name_preview)?;
    let front_matter = widgets::preview_text(&form.front_matter_preview)?;
    widgets::selectable(&front_matter, true)?;

    let preview_panel = widgets::vstack(CARD_GAP)?;
    widgets::add(&preview_panel, &titled_card("文件名预览", &file_name)?)?;
    widgets::add(
        &preview_panel,
        &titled_card("front matter 预览", &front_matter)?,
    )?;

    let content = widgets::grid(&[Length::Auto, Length::Auto], &[])?;
    content.SetRowSpacing(CARD_GAP)?;
    content.SetColumnSpacing(COLUMN_SPACING)?;
    widgets::append_column(&content, &Length::Star(FORM_WEIGHT))?;
    let preview_column = widgets::append_column(&content, &Length::Star(PREVIEW_WEIGHT))?;

    let forms = widgets::vstack(CARD_GAP)?;
    widgets::add(&forms, &basic_card(&title, &date, &time, &zone)?)?;
    widgets::add(&forms, &stacked_card("分类", &[&category1, &category2], 8.0)?)?;
    widgets::add(&forms, &tags_card(&tags, &extract)?)?;
    widgets::add(&forms, &authors_card(&authors_panel, &authors_summary)?)?;
    widgets::add(&forms, &titled_card("描述", &description)?)?;
    widgets::place(&forms, 0, 0)?;
    widgets::put(&content, &forms)?;
    widgets::place(&preview_panel, 0, 1)?;
    widgets::put(&content, &preview_panel)?;

    let commands = post_actions(Arc::clone(services), Arc::clone(actions_refresh))?;
    let header = page_header(&form.page_title, Some(&commands.root))?;

    Ok(Controls {
        header,
        content,
        preview_column,
        preview_panel,
        title,
        date,
        time,
        zone,
        category1,
        category2,
        tags,
        description,
        extract,
        authors: authors_panel,
        authors_summary,
        author_boxes: Arc::new(Mutex::new(Vec::new())),
        rendered_authors: Mutex::new(Vec::new()),
        file_name,
        front_matter,
        commands,
    })
}

/// 单个控件的卡片（[`super::section`] 的内容参数要求是面板）。
fn titled_card<P: Param<UIElement>>(title: &str, control: P) -> WinResult<Border> {
    let panel = widgets::vstack(0.0)?;
    widgets::add(&panel, control)?;
    section(title, "", &panel)
}

/// 竖排若干输入框的卡片。
fn stacked_card(title: &str, controls: &[&TextBox], spacing: f64) -> WinResult<Border> {
    let panel = widgets::vstack(spacing)?;
    for control in controls {
        widgets::add(&panel, *control)?;
    }
    section(title, "", &panel)
}

/// 标签卡：输入框 + 紧随其后的 `AI 提取关键字`（对偶原版同排的两颗元素）。
fn tags_card(tags: &TextBox, extract: &Button) -> WinResult<Border> {
    let panel = widgets::vstack(8.0)?;
    widgets::add(&panel, tags)?;
    widgets::add(&panel, extract)?;
    section("标签", "", &panel)
}

fn basic_card(title: &TextBox, date: &TextBox, time: &TextBox, zone: &ComboBox) -> WinResult<Border> {
    // 对偶 `ColumnDefinitions="*,Auto,Auto"`：日期占满剩余宽度，时间与时区贴右
    let row = widgets::grid(&[], &[Length::Star(1.0), Length::Auto, Length::Auto])?;
    row.SetColumnSpacing(8.0)?;
    widgets::place(date, 0, 0)?;
    widgets::put(&row, date)?;
    widgets::place(time, 0, 1)?;
    widgets::put(&row, time)?;
    widgets::place(zone, 0, 2)?;
    widgets::put(&row, zone)?;

    let panel = widgets::vstack(FIELD_SPACING)?;
    widgets::add(&panel, title)?;
    widgets::add(&panel, &row)?;
    section("基本信息", "", &panel)
}

fn authors_card(panel: &StackPanel, summary: &TextBlock) -> WinResult<Border> {
    let outer = widgets::vstack(8.0)?;
    widgets::add(&outer, panel)?;
    widgets::add(&outer, summary)?;
    section(
        "作者",
        "勾选本条博文的作者，候选来自项目根目录的 `authors.yml`。",
        &outer,
    )
}

/// 外层两行（页头 + 滚动内容），对偶原版 `Grid RowSpacing="16"`。
fn assemble(controls: &Controls) -> WinResult<Grid> {
    let root = widgets::grid(&[Length::Auto, Length::Star(1.0)], &[])?;
    root.SetRowSpacing(16.0)?;
    widgets::place(&controls.header.root, 0, 0)?;
    widgets::put(&root, &controls.header.root)?;

    let viewer = widgets::scroll_viewer(&controls.content)?;
    widgets::place(&viewer, 1, 0)?;
    widgets::put(&root, &viewer)?;
    Ok(root)
}

// ---------------------------------------------------------------- 事件接线

/// 输入实时写回编辑态（对偶 `UpdateSourceTrigger=PropertyChanged`），
/// 影响预览的字段顺带重启防抖。
fn wire_inputs(
    services: &Arc<Services>,
    controls: &Arc<Controls>,
    timer: &DispatcherQueueTimer,
) -> WinResult<()> {
    pull(&controls.title, services, timer, |form, value| {
        form.state.title = value;
    })?;
    pull(&controls.date, services, timer, |form, value| {
        form.set_date_text(&value);
    })?;
    pull(&controls.time, services, timer, |form, value| {
        form.set_time_text(&value);
    })?;

    // 主分类决定子分类可用性：原版靠 `NotifyPropertyChangedFor(HasCategory1)`，
    // 这里在写回之后直接改写那颗控件
    let category = controls.category2.clone();
    let field = controls.category1.clone();
    let gated = Arc::clone(services);
    let gate_timer = timer.clone();
    widgets::on_text_changed(&controls.category1, move || {
        gated.edit_form(|form| form.state.category1 = widgets::value_of(&field));
        let _ = widgets::enabled(&category, gated.form().has_category1());
        let _ = restart_timer(&gate_timer);
    })?;

    pull(&controls.category2, services, timer, |form, value| {
        form.state.category2 = value;
    })?;
    pull(&controls.tags, services, timer, |form, value| {
        form.state.tags = value;
    })?;
    pull(&controls.description, services, timer, |form, value| {
        form.state.description = value;
    })?;

    let services = Arc::clone(services);
    let timer = timer.clone();
    widgets::on_combo_changed(&controls.zone, move |index| {
        services.edit_form(|form| form.set_time_zone_index(index));
        let _ = restart_timer(&timer);
    })
}

/// 文本框 → 编辑态：读控件值、写状态、重启预览防抖。
fn pull<F>(
    target: &TextBox,
    services: &Arc<Services>,
    timer: &DispatcherQueueTimer,
    write: F,
) -> WinResult<()>
where
    F: Fn(&mut PostForm, String) + Send + 'static,
{
    let control = target.clone();
    let services = Arc::clone(services);
    let timer = timer.clone();
    widgets::on_text_changed(target, move || {
        let value = widgets::value_of(&control);
        services.edit_form(|form| write(form, value));
        let _ = restart_timer(&timer);
    })
}

/// 订阅页面尺寸（对偶 `SizeChanged` + `VisualStateManager.GoToState`）。
fn observe_width(page: &Page, controls: &Arc<Controls>) -> WinResult<()> {
    let observed = Arc::clone(controls);
    let element = page.clone().cast::<FrameworkElement>()?;
    on_size_changed(&element, move |width| {
        let _ = apply_width(&observed, width);
    })
}

/// 宽窗口左右分栏，窄窗口把预览压到下一行（与原版 `NarrowLayout` 的三个 setter 等价）。
fn apply_width(controls: &Controls, width: f64) -> WinResult<()> {
    let wide = width >= WIDE_LAYOUT_MIN_WIDTH;
    widgets::set_column_width(
        &controls.preview_column,
        &if wide {
            Length::Star(PREVIEW_WEIGHT)
        } else {
            Length::Pixel(0.0)
        },
    )?;
    controls.content.SetColumnSpacing(if wide { COLUMN_SPACING } else { 0.0 })?;
    widgets::place(
        &controls.preview_panel,
        if wide { 0 } else { 1 },
        if wide { 1 } else { 0 },
    )
}

// ---------------------------------------------------------------- 状态 → 控件

/// 把共享编辑态推回控件。每次动作收尾都经 `notify` 投回 UI 线程跑一遍。
///
/// `set_value` 会再触发一次 `TextChanged`，但写回的是同一个值、重启的也只是同一个防抖，
/// 不构成回环——与 .NET 侧双向 `x:Bind` 的写回行为一致。
fn push_to_controls(
    services: &Arc<Services>,
    controls: &Arc<Controls>,
    candidates: &PageState<Vec<Author>>,
    timer: &DispatcherQueueTimer,
) {
    let form = services.form();

    let _ = widgets::set_block_text(&controls.header.title, &form.page_title);
    let _ = widgets::set_value(&controls.title, &form.state.title);
    let _ = widgets::set_value(&controls.date, &form.date_text());
    let _ = widgets::set_value(&controls.time, &form.time_text());
    let _ = widgets::set_combo_index(&controls.zone, form.time_zone_index());
    let _ = widgets::set_value(&controls.category1, &form.state.category1);
    let _ = widgets::set_value(&controls.category2, &form.state.category2);
    let _ = widgets::set_value(&controls.tags, &form.state.tags);
    let _ = widgets::set_value(&controls.description, &form.state.description);

    let _ = widgets::enabled(&controls.category2, form.has_category1());
    let _ = widgets::enabled(&controls.extract, !form.is_extracting_keywords);
    // 对偶 `SavePostCommand.CanExecute`：保存期间三颗命令按钮一起禁用
    let _ = controls.commands.set_busy(form.is_busy);

    set_preview(&form, controls);
    let _ = widgets::set_block_text(&controls.authors_summary, &form.authors_display());
    let _ = sync_authors(controls, &held(candidates), &form.selected_author_ids, timer);
}

fn push_preview(controls: &Controls) {
    set_preview(&crate::services::services().form(), controls);
}

fn set_preview(form: &PostForm, controls: &Controls) {
    let _ = widgets::set_block_text(&controls.file_name, &form.file_name_preview);
    let _ = widgets::set_block_text(&controls.front_matter, &form.front_matter_preview);
}

/// 作者候选变化时重建勾选面板；候选没变时只回填勾选态。
///
/// 勾选态的唯一真相是 `PostForm::selected_author_ids`（对偶 .NET 重载后显式保留 `selectedIds`），
/// 所以刷新不会丢掉用户已勾的作者。
fn sync_authors(
    controls: &Arc<Controls>,
    candidates: &[Author],
    selected: &[String],
    timer: &DispatcherQueueTimer,
) -> WinResult<()> {
    let ids: Vec<String> = candidates
        .iter()
        .map(|author| author.id().to_owned())
        .collect();

    if rendered_authors(controls) != ids {
        rebuild_authors(controls, candidates, selected, timer)?;
        *controls
            .rendered_authors
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = ids;
        return Ok(());
    }

    for (id, target) in controls
        .author_boxes
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
    {
        let _ = widgets::set_checked(target, selected.iter().any(|picked| picked == id));
    }
    Ok(())
}

fn rebuild_authors(
    controls: &Arc<Controls>,
    candidates: &[Author],
    selected: &[String],
    timer: &DispatcherQueueTimer,
) -> WinResult<()> {
    widgets::clear_children(&controls.authors)?;
    let mut boxes = controls
        .author_boxes
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    boxes.clear();

    for author in candidates {
        let option = AuthorOption::from_author(author, selected);
        let target = widgets::check_box(&option.display_name, option.is_selected)?;
        widgets::on_check_changed(&target, selection_writer(&controls.author_boxes, timer))?;
        widgets::add(&controls.authors, &target)?;
        boxes.push((option.id, target));
    }
    Ok(())
}

/// 勾选变化 → 用当前全部勾选态覆盖 `selected_author_ids`，再重启防抖。
///
/// 整表覆盖而不是「增删一个 id」：与原版每次回调后重算 `BuildFormState` 同源，
/// 不会出现勾选态与编辑态各写一半的漂移。
///
/// 每个 `CheckBox` 各拿一份闭包（委托不是 `Clone`），但共享同一对句柄；
/// `Services` 从全局取，避免闭包反过来引用整棵控件树。
fn selection_writer(
    boxes: &Arc<Mutex<Vec<(String, CheckBox)>>>,
    timer: &DispatcherQueueTimer,
) -> impl Fn(bool) + Send + 'static {
    let boxes = Arc::clone(boxes);
    let timer = timer.clone();
    move |_| {
        let services = crate::services::services();
        services.edit_form(|form| form.set_selected_authors(checked_ids(&boxes)));
        let _ = restart_timer(&timer);
    }
}

fn checked_ids(boxes: &Mutex<Vec<(String, CheckBox)>>) -> Vec<String> {
    boxes
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .filter(|(_, target)| widgets::is_checked(target))
        .map(|(id, _)| id.clone())
        .collect()
}

fn rendered_authors(controls: &Controls) -> Vec<String> {
    controls
        .rendered_authors
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}
