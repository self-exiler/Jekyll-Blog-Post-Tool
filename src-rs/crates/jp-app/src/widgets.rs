//! 控件工厂：代码驱动 UI 的最小工具集（对偶 .NET 侧的 XAML 标记与样式）。
//!
//! 有意不做的事：winui3 投影里没有 `FrameworkElement::TryFindResource`，代码构建的控件树
//! 拿不到主题资源字典，因此卡片底色/描边/次要文字色用固定 ARGB 常量——半透明灰在
//! 深浅两套主题下都成立，代价是缺少主题切换时的精细还原。同理不套 `Style`
//! （`AccentButtonStyle` 等），主按钮与次按钮外观一致，只靠排布区分。
//!
//! 约定：凡是形参为 `Param<T>` 的工厂（`add`/`put`/`card`/`place`…）都按 **引用** 传子元素——
//! windows-core 只为 `&U`（`U: CanInto<T>`）实现了 `Param`，接口类型没有按值的转换，
//! 传属值会直接编译不过。
//!
//! 部分工厂（`toggle`/`radio`/`multiline_box`…）是控件工具箱的预留件，对偶 .NET 侧
//! 未被当前页面引用的样式资源，故允许暂未接线。
#![allow(dead_code)]

use windows::Foundation::{IReference, PropertyValue};
use windows::UI::{Color, Text::FontWeight};
use windows_core::{HSTRING, IInspectable, Interface, Param, Result as WinResult};

use winui3::Microsoft::UI::Xaml::{
    CornerRadius, FocusState, FrameworkElement, GridLength, GridUnitType, RoutedEventHandler,
    TextAlignment, TextWrapping, Thickness, UIElement, VerticalAlignment, Visibility,
};
use winui3::Microsoft::UI::Xaml::Controls::{
    Border, Button, CheckBox, ColumnDefinition, ComboBox, Control, Grid, ListView, Orientation,
    Panel, PasswordBox, RadioButton, RowDefinition, ScrollBarVisibility, ScrollMode, ScrollViewer,
    StackPanel, SelectionChangedEventHandler, TextBlock, TextBox, TextChangedEventHandler,
    ToggleSwitch,
};
use winui3::Microsoft::UI::Xaml::Media::{FontFamily, SolidColorBrush};

use crate::markdown_edit::{Edit, Selection};

/// 页标题字号（对偶 `TitleTextBlockStyle`）。
pub const TITLE_SIZE: f64 = 24.0;
/// 卡片小标题字号（对偶 `BodyStrongTextBlockStyle` 略放大）。
pub const SECTION_SIZE: f64 = 16.0;
/// 正文与控件字号：WinUI 默认值。
pub const BODY_SIZE: f64 = 14.0;
/// 次要说明文字字号。
pub const MUTED_SIZE: f64 = 13.0;

const NORMAL_WEIGHT: u16 = 400;
const SEMIBOLD_WEIGHT: u16 = 600;

/// `&str` → `HSTRING`（投影里所有字符串属性都吃 `&HSTRING`）。
pub fn hs(text: &str) -> HSTRING {
    HSTRING::from(text)
}

pub const fn thickness(all: f64) -> Thickness {
    Thickness {
        Left: all,
        Top: all,
        Right: all,
        Bottom: all,
    }
}

pub const fn insets(left: f64, top: f64, right: f64, bottom: f64) -> Thickness {
    Thickness {
        Left: left,
        Top: top,
        Right: right,
        Bottom: bottom,
    }
}

pub const fn uniform_radius(radius: f64) -> CornerRadius {
    CornerRadius {
        TopLeft: radius,
        TopRight: radius,
        BottomRight: radius,
        BottomLeft: radius,
    }
}

/// 等宽字体（Markdown 正文与预览用），对偶 XAML 里的 `FontFamily="Consolas"`。
pub fn mono_font() -> WinResult<FontFamily> {
    FontFamily::CreateInstanceWithName(&hs("Consolas"))
}

fn brush(color: Color) -> WinResult<SolidColorBrush> {
    SolidColorBrush::CreateInstanceWithColor(color)
}

/// 卡片底色：8% 不透明度的中性灰。
pub fn card_fill() -> WinResult<SolidColorBrush> {
    brush(Color {
        A: 0x14,
        R: 0x80,
        G: 0x80,
        B: 0x80,
    })
}

/// 卡片描边：12% 中性灰。
pub fn card_stroke() -> WinResult<SolidColorBrush> {
    brush(Color {
        A: 0x1F,
        R: 0x80,
        G: 0x80,
        B: 0x80,
    })
}

/// 次要文字色。
pub fn muted_fill() -> WinResult<SolidColorBrush> {
    brush(Color {
        A: 0xC8,
        R: 0x9A,
        G: 0x9A,
        B: 0x9A,
    })
}

/// 强调蓝（预览引用条），90% 不透明度在深浅主题下都可辨。
pub fn accent_fill() -> WinResult<SolidColorBrush> {
    brush(Color {
        A: 0xE6,
        R: 0x4C,
        G: 0x9E,
        B: 0xE8,
    })
}

// ---------------------------------------------------------------- 文本

fn text_at(text: &str, size: f64, weight: u16, muted: bool) -> WinResult<TextBlock> {
    let block = TextBlock::new()?;
    block.SetText(&hs(text))?;
    block.SetFontSize(size)?;
    block.SetFontWeight(FontWeight { Weight: weight })?;
    block.SetTextWrapping(TextWrapping::Wrap)?;
    if muted {
        block.SetForeground(&muted_fill()?)?;
    }
    Ok(block)
}

pub fn title_text(text: &str) -> WinResult<TextBlock> {
    text_at(text, TITLE_SIZE, SEMIBOLD_WEIGHT, false)
}

pub fn section_text(text: &str) -> WinResult<TextBlock> {
    text_at(text, SECTION_SIZE, SEMIBOLD_WEIGHT, false)
}

pub fn body_text(text: &str) -> WinResult<TextBlock> {
    text_at(text, BODY_SIZE, NORMAL_WEIGHT, false)
}

pub fn muted_text(text: &str) -> WinResult<TextBlock> {
    text_at(text, MUTED_SIZE, NORMAL_WEIGHT, true)
}

/// 只读预览：等宽 + 左上对齐，长行不撑破布局。
pub fn preview_text(text: &str) -> WinResult<TextBlock> {
    let block = text_at(text, BODY_SIZE, NORMAL_WEIGHT, false)?;
    block.SetFontFamily(&mono_font()?)?;
    block.SetVerticalAlignment(VerticalAlignment::Top)?;
    block.SetTextAlignment(TextAlignment::Left)?;
    Ok(block)
}

pub fn set_block_text(block: &TextBlock, text: &str) -> WinResult<()> {
    block.SetText(&hs(text))
}

/// 允许鼠标选中文字（对偶 `IsTextSelectionEnabled="True"`）——
/// front matter 预览要能整段复制出去，代码构建的控件树没有默认样式可借。
pub fn selectable(block: &TextBlock, enabled: bool) -> WinResult<()> {
    block.SetIsTextSelectionEnabled(enabled)
}

pub fn block_text(block: &TextBlock) -> String {
    block.Text().map(|text| text.to_string()).unwrap_or_default()
}

// ---------------------------------------------------------------- 面板

pub fn vstack(spacing: f64) -> WinResult<StackPanel> {
    let panel = StackPanel::new()?;
    panel.SetOrientation(Orientation::Vertical)?;
    panel.SetSpacing(spacing)?;
    Ok(panel)
}

pub fn hstack(spacing: f64) -> WinResult<StackPanel> {
    let panel = StackPanel::new()?;
    panel.SetOrientation(Orientation::Horizontal)?;
    panel.SetSpacing(spacing)?;
    Ok(panel)
}

pub fn add<P: Param<UIElement>>(panel: &StackPanel, child: P) -> WinResult<()> {
    panel.Children()?.Append(child)
}

/// 往任意 `Panel` 追加子元素：`StackPanel` 与 `Grid` 共用 `Panel::Children`。
/// 形参收 `&P` 而不是 `&Panel`：Rust 投影里没有隐式上转，调用方传 `&Grid` 时在此完成。
pub fn put<P, C>(panel: &P, child: C) -> WinResult<()>
where
    P: Interface + Clone,
    C: Param<UIElement>,
{
    panel.clone().cast::<Panel>()?.Children()?.Append(child)
}

/// 清空面板子元素（重建列表/作者区前先摘干净，`Panel::Children` 即 `UIElementCollection`）。
pub fn clear_children<P: Interface + Clone>(panel: &P) -> WinResult<()> {
    panel.clone().cast::<Panel>()?.Children()?.Clear()
}

pub fn add_all<I, P>(panel: &StackPanel, children: I) -> WinResult<()>
where
    I: IntoIterator<Item = P>,
    P: Param<UIElement>,
{
    let slot = panel.Children()?;
    for child in children {
        slot.Append(child)?;
    }
    Ok(())
}

/// 列/行高定义：与 XAML 的 `Auto` / `*` / 像素值一一对应。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Auto,
    Star(f64),
    Pixel(f64),
}

pub const fn grid_length(length: &Length) -> GridLength {
    let (value, unit) = match length {
        Length::Auto => (1.0, GridUnitType::Auto),
        Length::Star(weight) => (*weight, GridUnitType::Star),
        Length::Pixel(pixels) => (*pixels, GridUnitType::Pixel),
    };
    GridLength {
        Value: value,
        GridUnitType: unit,
    }
}

pub fn grid(rows: &[Length], columns: &[Length]) -> WinResult<Grid> {
    let panel = Grid::new()?;
    define_rows(&panel, rows)?;
    define_columns(&panel, columns)?;
    Ok(panel)
}

/// `ColumnDefinitions="280,*"` 的等价写法。
pub fn define_columns(panel: &Grid, widths: &[Length]) -> WinResult<()> {
    let definitions = panel.ColumnDefinitions()?;
    for width in widths {
        let definition = ColumnDefinition::new()?;
        definition.SetWidth(grid_length(width))?;
        definitions.Append(&definition)?;
    }
    Ok(())
}

pub fn define_rows(panel: &Grid, heights: &[Length]) -> WinResult<()> {
    let definitions = panel.RowDefinitions()?;
    for height in heights {
        let definition = RowDefinition::new()?;
        definition.SetHeight(grid_length(height))?;
        definitions.Append(&definition)?;
    }
    Ok(())
}

pub fn place<P>(element: P, row: i32, column: i32) -> WinResult<()>
where
    P: Param<FrameworkElement> + Clone,
{
    Grid::SetRow(element.clone(), row)?;
    Grid::SetColumn(element, column)
}

/// 追加一列并交出句柄：宽/窄布局要在运行期改它的 `Width`
/// （对偶 `Setter Target="PreviewColumn.Width"`）。
pub fn append_column(panel: &Grid, width: &Length) -> WinResult<ColumnDefinition> {
    let definition = ColumnDefinition::new()?;
    definition.SetWidth(grid_length(width))?;
    panel.ColumnDefinitions()?.Append(&definition)?;
    Ok(definition)
}

pub fn set_column_width(definition: &ColumnDefinition, width: &Length) -> WinResult<()> {
    definition.SetWidth(grid_length(width))
}

pub fn scroll_viewer<P: Param<IInspectable>>(content: P) -> WinResult<ScrollViewer> {
    let viewer = ScrollViewer::new()?;
    viewer.SetVerticalScrollMode(ScrollMode::Auto)?;
    viewer.SetVerticalScrollBarVisibility(ScrollBarVisibility::Auto)?;
    viewer.SetHorizontalScrollMode(ScrollMode::Disabled)?;
    viewer.SetHorizontalScrollBarVisibility(ScrollBarVisibility::Disabled)?;
    viewer.SetContent(content)?;
    Ok(viewer)
}

/// 横向滚动的容器（对偶工具栏那台 `ScrollViewer`：横滚开、纵滚关、滚动条隐藏）。
pub fn h_scroll_viewer<P: Param<IInspectable>>(content: P) -> WinResult<ScrollViewer> {
    let viewer = ScrollViewer::new()?;
    viewer.SetVerticalScrollMode(ScrollMode::Disabled)?;
    viewer.SetVerticalScrollBarVisibility(ScrollBarVisibility::Disabled)?;
    viewer.SetHorizontalScrollMode(ScrollMode::Auto)?;
    viewer.SetHorizontalScrollBarVisibility(ScrollBarVisibility::Hidden)?;
    viewer.SetContent(content)?;
    Ok(viewer)
}

/// 卡片容器：圆角 + 半透明底 + 1px 描边 + 16 内边距。
pub fn card<P: Param<UIElement>>(content: P) -> WinResult<Border> {
    let border = Border::new()?;
    border.SetPadding(thickness(16.0))?;
    border.SetCornerRadius(uniform_radius(8.0))?;
    border.SetBackground(&card_fill()?)?;
    border.SetBorderThickness(thickness(1.0))?;
    border.SetBorderBrush(&card_stroke()?)?;
    border.SetChild(content)?;
    Ok(border)
}

// ---------------------------------------------------------------- 可交互控件

pub fn button(label: &str) -> WinResult<Button> {
    let target = Button::new()?;
    target.SetContent(&body_text(label)?)?;
    Ok(target)
}

/// 按钮点击：处理器在 UI 线程执行，耗时动作请自行 `spawn_work`。
pub fn on_click<F>(target: &Button, handler: F) -> WinResult<()>
where
    F: Fn() + Send + 'static,
{
    let sink = RoutedEventHandler::new(move |_, _| {
        handler();
        Ok(())
    });
    target.Click(&sink).map(|_| ())
}

pub fn toggle(label: &str, is_on: bool) -> WinResult<ToggleSwitch> {
    let target = ToggleSwitch::new()?;
    target.SetHeader(&body_text(label)?)?;
    target.SetIsOn(is_on)?;
    Ok(target)
}

/// 开关状态变化。注意：程序 `SetIsOn` 同样触发，回填初值前请先摘掉监听。
pub fn on_toggled<F>(target: &ToggleSwitch, handler: F) -> WinResult<()>
where
    F: Fn(bool) + Send + 'static,
{
    let sink = RoutedEventHandler::new(move |sender, _| {
        if let Some(sender) = sender.as_ref() {
            if let Ok(target) = sender.cast::<ToggleSwitch>() {
                if let Ok(is_on) = target.IsOn() {
                    handler(is_on);
                }
            }
        }
        Ok(())
    });
    target.Toggled(&sink).map(|_| ())
}

pub fn text_box(header: &str, value: &str) -> WinResult<TextBox> {
    let target = TextBox::new()?;
    if !header.is_empty() {
        target.SetHeader(&body_text(header)?)?;
    }
    target.SetText(&hs(value))?;
    Ok(target)
}

/// 多行输入：接受回车、软换行，最小高度按行数估算（约 20px/行 + 32px 内边距）。
pub fn multiline_box(header: &str, value: &str, lines: f64) -> WinResult<TextBox> {
    let target = text_box(header, value)?;
    target.SetAcceptsReturn(true)?;
    target.SetTextWrapping(TextWrapping::Wrap)?;
    target.SetFontFamily(&mono_font()?)?;
    target.SetMinHeight(lines * 20.0 + 32.0)?;
    Ok(target)
}

pub fn value_of(target: &TextBox) -> String {
    target.Text().map(|text| text.to_string()).unwrap_or_default()
}

/// 摘要输入：`description` 是人读文本，不套等宽字体，只保留「可回车 + 软换行 + 100px 起高」
/// （对偶 `MinHeight="100" AcceptsReturn="True" TextWrapping="Wrap"`）。
pub fn description_box(header: &str, value: &str) -> WinResult<TextBox> {
    let target = text_box(header, value)?;
    target.SetAcceptsReturn(true)?;
    target.SetTextWrapping(TextWrapping::Wrap)?;
    target.SetMinHeight(100.0)?;
    Ok(target)
}

pub fn set_value(target: &TextBox, text: &str) -> WinResult<()> {
    target.SetText(&hs(text))
}

pub fn read_only(target: &TextBox, locked: bool) -> WinResult<()> {
    target.SetIsReadOnly(locked)
}

pub fn placeholder(target: &TextBox, text: &str) -> WinResult<()> {
    target.SetPlaceholderText(&hs(text))
}

/// 正文编辑器：等宽、不换行、可回车，纵向滚动交给控件内建的 ScrollViewer。
/// 对偶 `BodyTextBox`（`AcceptsReturn` + `TextWrapping="NoWrap"` + `MinHeight="240"`）。
pub fn body_editor(hint: &str) -> WinResult<TextBox> {
    let target = TextBox::new()?;
    target.SetAcceptsReturn(true)?;
    target.SetTextWrapping(TextWrapping::NoWrap)?;
    target.SetFontFamily(&mono_font()?)?;
    target.SetMinHeight(240.0)?;
    placeholder(&target, hint)?;
    Ok(target)
}

/// 让控件取得键盘焦点（工具栏改完正文后把光标还给编辑器）。
pub fn focus<I: Interface + Clone>(element: &I) -> WinResult<()> {
    element
        .clone()
        .cast::<UIElement>()?
        .Focus(FocusState::Programmatic)
        .map(|_| ())
}

/// 控件快照：编辑器纯函数（`markdown_edit`）的输入。
pub fn snapshot(target: &TextBox) -> WinResult<Selection> {
    let text = target.Text()?;
    let start = target.SelectionStart()?;
    let length = target.SelectionLength()?;
    Ok(Selection::new(text.to_string().as_str(), start, length))
}

/// 光标位置（`SelectionStart`，UTF-16 码元计数）；读不到或为负时按 `None`（追加到末尾）。
pub fn caret_of(target: &TextBox) -> Option<usize> {
    target.SelectionStart().ok().filter(|start| *start >= 0).map(|start| start as usize)
}

/// 把一次编辑结果写回控件：内容 + 光标/选区。
pub fn apply_edit(target: &TextBox, edit: &Edit) -> WinResult<()> {
    target.SetText(&edit.text_hstring())?;
    target.SetSelectionStart(edit.selection_start as i32)?;
    target.SetSelectionLength(edit.selection_length as i32)
}

pub fn on_text_changed<F>(target: &TextBox, handler: F) -> WinResult<()>
where
    F: Fn() + Send + 'static,
{
    let sink = TextChangedEventHandler::new(move |_, _| {
        handler();
        Ok(())
    });
    target.TextChanged(&sink).map(|_| ())
}

// ---------------------------------------------------------------- 选项与密码

/// 装箱成 `IInspectable`：`Items`/`SetTitle` 这类形参是 `IInspectable`，投影不接受裸字符串。
pub fn boxed_str(text: &str) -> WinResult<IInspectable> {
    PropertyValue::CreateString(&hs(text))
}

/// 装箱勾选态。必须走 `PropertyValue::CreateBoolean`（combase 的标准装箱对象）：
/// `Reference::new` 造的纯 Rust `IReference<bool>` 会被 XAML 属性系统拒绝，
/// `SetIsChecked` 直接 E_FAIL（实测 WinAppSDK 2.5；错误消息指向 LimitedAccessFeatures，是误导）。
fn bool_reference(checked: bool) -> WinResult<IReference<bool>> {
    PropertyValue::CreateBoolean(checked)?.cast::<IReference<bool>>()
}

pub fn check_box(label: &str, checked: bool) -> WinResult<CheckBox> {
    let target = CheckBox::new()?;
    target.SetContent(&body_text(label)?)?;
    target.SetIsChecked(&bool_reference(checked)?)?;
    Ok(target)
}

pub fn radio(label: &str, checked: bool) -> WinResult<RadioButton> {
    let target = RadioButton::new()?;
    target.SetContent(&body_text(label)?)?;
    target.SetIsChecked(&bool_reference(checked)?)?;
    Ok(target)
}

pub fn is_checked(target: &CheckBox) -> bool {
    checked_state(&target.IsChecked())
}

pub fn set_checked(target: &CheckBox, checked: bool) -> WinResult<()> {
    target.SetIsChecked(&bool_reference(checked)?)
}

pub fn is_selected(target: &RadioButton) -> bool {
    checked_state(&target.IsChecked())
}

pub fn set_selected(target: &RadioButton, checked: bool) -> WinResult<()> {
    target.SetIsChecked(&bool_reference(checked)?)
}

/// `IsChecked` 是三态可空值：读不到（未初始化/取属性失败）一律按未勾选。
fn checked_state(value: &WinResult<IReference<bool>>) -> bool {
    value
        .as_ref()
        .ok()
        .and_then(|boxed| boxed.Value().ok())
        .unwrap_or(false)
}

/// 勾选态变化：`Click` 对勾选与取消都触发，读一次当前态即可，不必分别接 `Checked`/`Unchecked`。
pub fn on_check_changed<F>(target: &CheckBox, handler: F) -> WinResult<()>
where
    F: Fn(bool) + Send + 'static,
{
    let observed = target.clone();
    let sink = RoutedEventHandler::new(move |_, _| {
        handler(is_checked(&observed));
        Ok(())
    });
    target.Click(&sink).map(|_| ())
}

/// 单选：只在选中时回调（RadioButton 不会因点别的项而触发自己）。
pub fn on_radio_selected<F>(target: &RadioButton, handler: F) -> WinResult<()>
where
    F: Fn() + Send + 'static,
{
    let observed = target.clone();
    let sink = RoutedEventHandler::new(move |_, _| {
        if is_selected(&observed) {
            handler();
        }
        Ok(())
    });
    target.Click(&sink).map(|_| ())
}

pub fn password_box(header: &str) -> WinResult<PasswordBox> {
    let target = PasswordBox::new()?;
    if !header.is_empty() {
        target.SetHeader(&body_text(header)?)?;
    }
    Ok(target)
}

pub fn password_of(target: &PasswordBox) -> String {
    target.Password().map(|text| text.to_string()).unwrap_or_default()
}

pub fn set_password(target: &PasswordBox, text: &str) -> WinResult<()> {
    target.SetPassword(&hs(text))
}

pub fn on_password_changed<F>(target: &PasswordBox, handler: F) -> WinResult<()>
where
    F: Fn() + Send + 'static,
{
    let sink = RoutedEventHandler::new(move |_, _| {
        handler();
        Ok(())
    });
    target.PasswordChanged(&sink).map(|_| ())
}

/// 下拉选择。选项是字符串，页面按 **下标** 取值（避免从 `SelectedItem` 反解 `IInspectable`）。
pub fn combo_box(header: &str, options: &[String], selected: Option<usize>) -> WinResult<ComboBox> {
    let target = ComboBox::new()?;
    if !header.is_empty() {
        target.SetHeader(&body_text(header)?)?;
    }

    let items = target.Items()?;
    for option in options {
        let boxed = boxed_str(option)?;
        items.Append(&boxed)?;
    }
    target.SetSelectedIndex(selected.map_or(-1, |index| index as i32))?;
    Ok(target)
}

/// 下拉当前下标；未选中（`-1`）返回 `None`。
pub fn combo_index(target: &ComboBox) -> Option<usize> {
    target.SelectedIndex().ok().filter(|index| *index >= 0).map(|index| index as usize)
}

pub fn set_combo_index(target: &ComboBox, index: Option<usize>) -> WinResult<()> {
    target.SetSelectedIndex(index.map_or(-1, |index| index as i32))
}

pub fn on_combo_changed<F>(target: &ComboBox, handler: F) -> WinResult<()>
where
    F: Fn(Option<usize>) + Send + 'static,
{
    let sink = SelectionChangedEventHandler::new(move |sender, _| {
        let index = sender
            .as_ref()
            .and_then(|sender| sender.cast::<ComboBox>().ok())
            .and_then(|target| combo_index(&target));
        handler(index);
        Ok(())
    });
    target.SelectionChanged(&sink).map(|_| ())
}

// ---------------------------------------------------------------- 列表

pub fn list_view() -> WinResult<ListView> {
    ListView::new()
}

/// 整体重写列表内容：文本项直接作为 item 塞进 `Items`（投影里没有 `DataTemplate` 可用，
/// 也就不用 `ItemsSource`）。返回后选中项被清空，调用方按状态重新指定下标。
pub fn fill_list(target: &ListView, texts: &[String]) -> WinResult<()> {
    let items = target.Items()?;
    items.Clear()?;
    for text in texts {
        let label = body_text(text)?;
        items.Append(&label)?;
    }
    Ok(())
}

pub fn list_size(target: &ListView) -> usize {
    target.Items().ok().and_then(|items| items.Size().ok()).unwrap_or(0) as usize
}

pub fn list_index(target: &ListView) -> Option<usize> {
    target.SelectedIndex().ok().filter(|index| *index >= 0).map(|index| index as usize)
}

/// `-1` 表示清空选中（对偶 `SelectedItem = null`）。
pub fn set_list_index(target: &ListView, index: Option<usize>) -> WinResult<()> {
    target.SetSelectedIndex(index.map_or(-1, |index| index as i32))
}

/// 列表选中项变化。程序化 `SetSelectedIndex` 同样触发——页面的回填是幂等的，
/// 按状态重设同一行不会改变结果。
pub fn on_list_changed<F>(target: &ListView, handler: F) -> WinResult<()>
where
    F: Fn(Option<usize>) + Send + 'static,
{
    let sink = SelectionChangedEventHandler::new(move |sender, _| {
        let index = sender
            .as_ref()
            .and_then(|sender| sender.cast::<ListView>().ok())
            .and_then(|target| list_index(&target));
        handler(index);
        Ok(())
    });
    target.SelectionChanged(&sink).map(|_| ())
}

// ---------------------------------------------------------------- 通用属性

pub fn margin<I: Interface + Clone>(element: &I, value: Thickness) -> WinResult<()> {
    element.clone().cast::<FrameworkElement>()?.SetMargin(value)
}

pub fn min_width<I: Interface + Clone>(element: &I, value: f64) -> WinResult<()> {
    element.clone().cast::<FrameworkElement>()?.SetMinWidth(value)
}

pub fn min_height<I: Interface + Clone>(element: &I, value: f64) -> WinResult<()> {
    element.clone().cast::<FrameworkElement>()?.SetMinHeight(value)
}

pub fn padding<C: Interface + Clone>(control: &C, value: Thickness) -> WinResult<()> {
    control.clone().cast::<Control>()?.SetPadding(value)
}

pub fn visible<I: Interface + Clone>(element: &I, shown: bool) -> WinResult<()> {
    element.clone().cast::<UIElement>()?.SetVisibility(if shown {
        Visibility::Visible
    } else {
        Visibility::Collapsed
    })
}

pub fn enabled<C: Interface + Clone>(control: &C, usable: bool) -> WinResult<()> {
    control.clone().cast::<Control>()?.SetIsEnabled(usable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_map_to_grid_units() {
        assert_eq!(
            grid_length(&Length::Auto),
            GridLength {
                Value: 1.0,
                GridUnitType: GridUnitType::Auto
            }
        );
        assert_eq!(
            grid_length(&Length::Star(2.0)),
            GridLength {
                Value: 2.0,
                GridUnitType: GridUnitType::Star
            }
        );
        assert_eq!(
            grid_length(&Length::Pixel(280.0)),
            GridLength {
                Value: 280.0,
                GridUnitType: GridUnitType::Pixel
            }
        );
    }

    #[test]
    fn thickness_helpers_are_uniform_or_explicit() {
        assert_eq!(thickness(16.0), insets(16.0, 16.0, 16.0, 16.0));
        assert_eq!(
            insets(1.0, 2.0, 3.0, 4.0),
            Thickness {
                Left: 1.0,
                Top: 2.0,
                Right: 3.0,
                Bottom: 4.0
            }
        );
    }

    #[test]
    fn card_radius_is_uniform() {
        assert_eq!(
            uniform_radius(8.0),
            CornerRadius {
                TopLeft: 8.0,
                TopRight: 8.0,
                BottomRight: 8.0,
                BottomLeft: 8.0
            }
        );
    }
}
