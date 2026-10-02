//! md_render 块树 → 预览控件树（纯渲染层：只造控件，不含解析与状态）。
//!
//! 只用 TextBlock/Border/StackPanel/Grid 这些基础控件做「近似渲染」，替代 .NET 侧的
//! `MarkdownTextBlock`（CommunityToolkit 控件不在 Rust 投影里）：标题走字号+字重、
//! 粗斜体/删除线走 `Documents` 内联元素、链接用 `Hyperlink`+`NavigateUri`（点击由系统开浏览器）、
//! 图片降级为灰色 `[图片]` 占位。渲染目标读通即可，不追求与 GitHub 渲染像素一致。

use windows::Foundation::Uri;
use windows::UI::Text::{FontWeight, TextDecorations};
use windows_core::{Interface, Result as WinResult};

use winui3::Microsoft::UI::Xaml::{
    Controls::{Border, Grid, StackPanel, TextBlock},
    Documents::{Bold, Hyperlink, InlineCollection, Italic, LineBreak, Run},
    FrameworkElement, TextWrapping, UIElement,
};

use crate::md_render::{self, Block, Inline};
use crate::widgets;
use crate::widgets::Length;

/// 标题字号（1~6 级；0 号位占位不用）。
const HEADING_SIZES: [f64; 7] = [0.0, 22.0, 19.0, 17.0, 15.0, 14.0, 13.0];
const HEADING_WEIGHT: u16 = 600;
/// 列表嵌套时每层缩进。
const LIST_INDENT: f64 = 16.0;

/// 把块树渲染进纵向面板（调用方先清空面板）。
pub fn render(blocks: &[Block], panel: &StackPanel) -> WinResult<()> {
    for block in blocks {
        if let Some(element) = block_control(block, 0)? {
            widgets::add(panel, &element)?;
        }
    }
    Ok(())
}

fn heading_size(level: u8) -> f64 {
    let index = usize::from(level).clamp(1, 6);
    HEADING_SIZES[index]
}

fn block_control(block: &Block, depth: u8) -> WinResult<Option<UIElement>> {
    match block {
        Block::Heading { level, inlines } => {
            let text = paragraph_shell()?;
            text.SetFontSize(heading_size(*level))?;
            text.SetFontWeight(FontWeight { Weight: HEADING_WEIGHT })?;
            fill_inlines(&text.Inlines()?, inlines)?;
            Ok(Some(upcast(&text)?))
        }
        Block::Paragraph(inlines) => {
            let text = paragraph_shell()?;
            fill_inlines(&text.Inlines()?, inlines)?;
            Ok(Some(upcast(&text)?))
        }
        Block::Code(code) => Ok(Some(upcast(&code_block(code)?)?)),
        Block::Quote(children) => Ok(Some(upcast(&quote_block(children, depth)?)?)),
        Block::List {
            ordered,
            start,
            items,
        } => Ok(Some(upcast(&list_block(*ordered, *start, items, depth)?)?)),
        Block::Table(rows) => Ok(Some(upcast(&table_block(rows)?)?)),
        Block::Rule => Ok(Some(upcast(&rule()?)?)),
    }
}

/// 预览段落的公共设置：软换行 + 可整段选中复制。
fn paragraph_shell() -> WinResult<TextBlock> {
    let text = TextBlock::new()?;
    text.SetTextWrapping(TextWrapping::Wrap)?;
    text.SetIsTextSelectionEnabled(true)?;
    Ok(text)
}

fn upcast<T: Interface + Clone>(element: &T) -> WinResult<UIElement> {
    element.clone().cast::<UIElement>()
}

/// 代码块：卡片描边 + 等宽原文。
fn code_block(code: &str) -> WinResult<Border> {
    let text = paragraph_shell()?;
    text.SetText(&widgets::hs(code))?;
    text.SetFontFamily(&widgets::mono_font()?)?;

    let border = Border::new()?;
    border.SetPadding(widgets::insets(10.0, 8.0, 10.0, 8.0))?;
    border.SetCornerRadius(widgets::uniform_radius(6.0))?;
    border.SetBackground(&widgets::card_fill()?)?;
    border.SetBorderThickness(widgets::thickness(1.0))?;
    border.SetBorderBrush(&widgets::card_stroke()?)?;
    border.SetChild(&text)?;
    Ok(border)
}

/// 引用块：左侧 3px 蓝色竖条 + 子块纵向排布。
fn quote_block(children: &[Block], depth: u8) -> WinResult<Grid> {
    let inner = widgets::vstack(6.0)?;
    render(children, &inner)?;
    widgets::margin(&inner, widgets::insets(10.0, 0.0, 0.0, 0.0))?;

    let bar = Border::new()?;
    bar.SetWidth(3.0)?;
    bar.SetBackground(&widgets::accent_fill()?)?;

    let grid = widgets::grid(&[], &[Length::Auto, Length::Star(1.0)])?;
    widgets::place(&bar, 0, 0)?;
    widgets::put(&grid, &bar)?;
    widgets::place(&inner, 0, 1)?;
    widgets::put(&grid, &inner)?;
    if depth > 0 {
        widgets::margin(&grid, widgets::insets(LIST_INDENT * f64::from(depth), 0.0, 0.0, 0.0))?;
    }
    Ok(grid)
}

/// 列表：每项一行「前缀 + 内容」，条目内块（含嵌套列表）递归渲染。
fn list_block(ordered: bool, start: u64, items: &[Vec<Block>], depth: u8) -> WinResult<StackPanel> {
    let panel = widgets::vstack(4.0)?;
    for (index, item) in items.iter().enumerate() {
        let prefix = if ordered {
            format!("{}.", start + index as u64)
        } else {
            "•".to_owned()
        };
        let bullet = widgets::muted_text(&prefix)?;
        widgets::margin(&bullet, widgets::insets(0.0, 0.0, 8.0, 0.0))?;

        let content = item_content(item, depth)?;
        // 块产物都以 UIElement 交出，挂网格列要回 FrameworkElement（预览控件全是 FE）
        let content = content.cast::<FrameworkElement>()?;
        let row = widgets::grid(&[], &[Length::Auto, Length::Star(1.0)])?;
        widgets::place(&bullet, 0, 0)?;
        widgets::put(&row, &bullet)?;
        widgets::place(&content, 0, 1)?;
        widgets::put(&row, &content)?;
        widgets::add(&panel, &row)?;
    }
    if depth > 0 {
        widgets::margin(&panel, widgets::insets(LIST_INDENT, 0.0, 0.0, 0.0))?;
    }
    Ok(panel)
}

/// 列表条目内容：单个段落直接摊平成一块文本（紧凑列表的常见形态），否则纵向堆。
fn item_content(item: &[Block], depth: u8) -> WinResult<UIElement> {
    if let [only] = item {
        if let Some(element) = block_control(only, depth + 1)? {
            return Ok(element);
        }
    }
    let panel = widgets::vstack(4.0)?;
    render(item, &panel)?;
    upcast(&panel)
}

/// 表格：降级为「单元格 | 单元格」逐行文本，首行加粗当表头。
fn table_block(rows: &[Vec<Vec<Inline>>]) -> WinResult<StackPanel> {
    let panel = widgets::vstack(2.0)?;
    for (index, row) in rows.iter().enumerate() {
        let line = row
            .iter()
            .map(|cell| md_render::plain_text(cell))
            .collect::<Vec<_>>()
            .join("  |  ");
        let text = paragraph_shell()?;
        text.SetText(&widgets::hs(&line))?;
        if index == 0 {
            text.SetFontWeight(FontWeight { Weight: HEADING_WEIGHT })?;
        }
        widgets::add(&panel, &text)?;
    }
    Ok(panel)
}

fn rule() -> WinResult<Border> {
    let line = Border::new()?;
    line.SetHeight(1.0)?;
    line.SetBackground(&widgets::card_stroke()?)?;
    widgets::margin(&line, widgets::insets(0.0, 6.0, 0.0, 6.0))?;
    Ok(line)
}

// ---------------------------------------------------------------- 内联

fn fill_inlines(collection: &InlineCollection, inlines: &[Inline]) -> WinResult<()> {
    for inline in inlines {
        match inline {
            Inline::Text(text) => collection.Append(&run(text, None, None)?)?,
            Inline::Code(text) => {
                collection.Append(&run(text, Some(&widgets::mono_font()?), Some(&widgets::muted_fill()?))?)?;
            }
            Inline::Break => collection.Append(&LineBreak::new()?)?,
            Inline::Bold(inner) => {
                let element = Bold::new()?;
                fill_inlines(&element.Inlines()?, inner)?;
                collection.Append(&element)?;
            }
            Inline::Italic(inner) => {
                let element = Italic::new()?;
                fill_inlines(&element.Inlines()?, inner)?;
                collection.Append(&element)?;
            }
            Inline::Strike(inner) => {
                // Span 不可激活构造，删除线按整段文本降级挂在一个 Run 上
                let element = run(&md_render::plain_text(inner), None, None)?;
                element.SetTextDecorations(TextDecorations::Strikethrough)?;
                collection.Append(&element)?;
            }
            Inline::Link { text, url } => {
                let link = Hyperlink::new()?;
                fill_inlines(&link.Inlines()?, text)?;
                if let Ok(uri) = Uri::CreateUri(&widgets::hs(url)) {
                    let _ = link.SetNavigateUri(&uri);
                }
                collection.Append(&link)?;
            }
            Inline::Image { alt, .. } => {
                let label = if alt.is_empty() {
                    "[图片]".to_owned()
                } else {
                    format!("[图片: {alt}]")
                };
                collection.Append(&run(&label, None, Some(&widgets::muted_fill()?))?)?;
            }
        }
    }
    Ok(())
}

fn run(
    text: &str,
    family: Option<&winui3::Microsoft::UI::Xaml::Media::FontFamily>,
    foreground: Option<&winui3::Microsoft::UI::Xaml::Media::SolidColorBrush>,
) -> WinResult<Run> {
    let element = Run::new()?;
    element.SetText(&widgets::hs(text))?;
    if let Some(family) = family {
        element.SetFontFamily(family)?;
    }
    if let Some(foreground) = foreground {
        element.SetForeground(foreground)?;
    }
    Ok(element)
}
