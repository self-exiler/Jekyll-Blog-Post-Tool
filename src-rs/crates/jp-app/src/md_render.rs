//! Markdown → 预览块模型（纯数据，无 UI 依赖，可单测）。
//!
//! `post_body` 页把 [`Block`] 树映射成控件；这里只负责把 pulldown-cmark 事件流
//! 收敛成一套够渲染用的简化结构。刻意不追求 CommonMark 全量语义：
//! 预览的目标是「所见接近所存」，脚注/HTML 等按纯文本降级，保证永不空白。

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Code(String),
    /// 段内换行（硬换行）。软换行按空格并入 [`Inline::Text`]。
    Break,
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Strike(Vec<Inline>),
    Link { text: Vec<Inline>, url: String },
    Image { alt: String, url: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading { level: u8, inlines: Vec<Inline> },
    Paragraph(Vec<Inline>),
    /// 围栏/缩进代码块原文（不含 info 串）。
    Code(String),
    Quote(Vec<Block>),
    List {
        ordered: bool,
        start: u64,
        /// 每个条目自身是一串块（可含嵌套列表）。
        items: Vec<Vec<Block>>,
    },
    /// 表格扁平化为「行 → 单元格内联」，首行即表头。
    Table(Vec<Vec<Vec<Inline>>>),
    Rule,
}

/// 把 markdown 解析成块树。CommonMark 对任意输入都有定义，解析不会失败；
/// 未覆盖的结构（脚注定义、HTML）降级为文本。
pub fn parse(markdown: &str) -> Vec<Block> {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;

    let mut machine = Machine::default();
    for event in Parser::new_ext(markdown, options) {
        machine.event(event);
    }
    machine.finish()
}

#[derive(Debug)]
enum Container {
    Root(Vec<Block>),
    /// 段落/标题/单元格的内联收集标记，实际内容在 `sinks` 栈上
    Inline(InlineOwner),
    Quote(Vec<Block>),
    Item(Vec<Block>),
    List {
        ordered: bool,
        start: u64,
        items: Vec<Vec<Block>>,
    },
    Code(String),
    Row(Vec<Vec<Inline>>),
    Table(Vec<Vec<Vec<Inline>>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineOwner {
    Heading(HeadingLevel),
    Paragraph,
    TableCell,
    /// 未覆盖结构（脚注定义等）：按段落降级
    Other,
}

#[derive(Debug)]
enum Wrap {
    Strong,
    Emphasis,
    Strike,
    Link { url: String },
    Image { url: String },
}

#[derive(Debug)]
struct Machine {
    containers: Vec<Container>,
    /// 内联收集栈：最后一个 sink 接收文本事件；包裹型内联压新 sink
    sinks: Vec<Vec<Inline>>,
    wraps: Vec<Wrap>,
}

impl Machine {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(end) => self.end(end),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => self.push_inline(Inline::Code(code.to_string())),
            Event::SoftBreak => self.text(" "),
            Event::HardBreak => self.push_inline(Inline::Break),
            Event::Rule => self.push_block(Block::Rule),
            Event::TaskListMarker(checked) => {
                self.text(if checked { "[x] " } else { "[ ] " })
            }
            Event::Html(html) | Event::InlineHtml(html) => self.text(html.trim()),
            Event::FootnoteReference(name) => self.text(&format!("[^{name}]")),
            Event::InlineMath(tex) | Event::DisplayMath(tex) => self.text(&tex.to_string()),
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Heading { level, .. } => self.start_inline(InlineOwner::Heading(level)),
            Tag::Paragraph => {
                // 松弛列表项里，先行内容（如任务标记）可能已在隐式收集器中，并进真段落
                let mut carried = match self.containers.last() {
                    Some(Container::Item(_)) if self.sinks.len() == 1 => self.pop_sink(),
                    _ => Vec::new(),
                };
                self.start_inline(InlineOwner::Paragraph);
                if !carried.is_empty() {
                    if let Some(sink) = self.sinks.last_mut() {
                        sink.append(&mut carried);
                    }
                }
            }
            Tag::BlockQuote(_) => self.containers.push(Container::Quote(Vec::new())),
            Tag::List(start) => self.containers.push(Container::List {
                ordered: start.is_some(),
                start: start.unwrap_or(1),
                items: Vec::new(),
            }),
            Tag::Item => self.containers.push(Container::Item(Vec::new())),
            Tag::CodeBlock(_) => self.containers.push(Container::Code(String::new())),
            Tag::Strong => self.push_wrap(Wrap::Strong),
            Tag::Emphasis => self.push_wrap(Wrap::Emphasis),
            Tag::Strikethrough => self.push_wrap(Wrap::Strike),
            Tag::Link { dest_url, .. } => self.push_wrap(Wrap::Link {
                url: dest_url.to_string(),
            }),
            Tag::Image { dest_url, .. } => self.push_wrap(Wrap::Image {
                url: dest_url.to_string(),
            }),
            Tag::Table(_) => self.containers.push(Container::Table(Vec::new())),
            Tag::TableHead | Tag::TableRow => self.containers.push(Container::Row(Vec::new())),
            Tag::TableCell => self.start_inline(InlineOwner::TableCell),
            _ => self.start_inline(InlineOwner::Other),
        }
    }

    fn end(&mut self, end: TagEnd) {
        match end {
            TagEnd::Heading(level) => {
                let inlines = self.pop_sink();
                self.expect_inline_marker(InlineOwner::Heading(level));
                if !inlines.is_empty() {
                    self.push_block(Block::Heading {
                        level: heading_level(level),
                        inlines,
                    });
                }
            }
            TagEnd::Paragraph => {
                let inlines = self.pop_sink();
                self.expect_inline_marker(InlineOwner::Paragraph);
                self.push_paragraph(inlines);
            }
            TagEnd::TableCell => {
                let cell = self.pop_sink();
                self.expect_inline_marker(InlineOwner::TableCell);
                if let Some(Container::Row(rows)) = self.containers.last_mut() {
                    rows.push(cell);
                }
            }
            TagEnd::TableRow | TagEnd::TableHead => {
                if let Some(Container::Row(cells)) = self.containers.pop() {
                    if let Some(Container::Table(table)) = self.containers.last_mut() {
                        table.push(cells);
                    }
                }
            }
            TagEnd::Table => {
                if let Some(Container::Table(rows)) = self.containers.pop() {
                    self.push_block(Block::Table(rows));
                }
            }
            TagEnd::BlockQuote(_) => {
                if let Some(Container::Quote(blocks)) = self.containers.pop() {
                    self.push_block(Block::Quote(blocks));
                }
            }
            TagEnd::List(_) => {
                if let Some(Container::List {
                    ordered,
                    start,
                    items,
                }) = self.containers.pop()
                {
                    self.push_block(Block::List { ordered, start, items });
                }
            }
            TagEnd::Item => {
                // 收掉项内没有 Paragraph 标签收尾的隐式文本
                if matches!(self.containers.last(), Some(Container::Item(_))) && !self.sinks.is_empty()
                {
                    let inlines = self.pop_sink();
                    self.push_paragraph(inlines);
                }
                if let Some(Container::Item(blocks)) = self.containers.pop() {
                    if let Some(Container::List { items, .. }) = self.containers.last_mut() {
                        items.push(blocks);
                    }
                }
            }
            TagEnd::CodeBlock => {
                if let Some(Container::Code(text)) = self.containers.pop() {
                    self.push_block(Block::Code(text.trim_end().to_owned()));
                }
            }
            TagEnd::Strong
            | TagEnd::Emphasis
            | TagEnd::Strikethrough
            | TagEnd::Link
            | TagEnd::Image => self.pop_wrap(),
            _ => {
                let inlines = self.pop_sink();
                self.expect_inline_marker(InlineOwner::Other);
                self.push_paragraph(inlines);
            }
        }
    }

    /// 输入结束：把没闭合的容器全部折叠回根（防御截断/畸形事件流）。
    fn finish(mut self) -> Vec<Block> {
        while let Some(container) = self.containers.pop() {
            match container {
                Container::Root(blocks) => {
                    // 根出栈即收工（重新压回避免下面的分支吞掉它）
                    self.containers.push(Container::Root(blocks));
                    break;
                }
                Container::Quote(blocks) | Container::Item(blocks) => self.push_blocks(blocks),
                Container::List { items, .. } => {
                    for item in items {
                        self.push_blocks(item);
                    }
                }
                Container::Code(text) => {
                    if !text.trim().is_empty() {
                        self.push_block_raw(Block::Code(text.trim_end().to_owned()));
                    }
                }
                Container::Row(cells) => {
                    for cell in cells {
                        self.push_paragraph(cell);
                    }
                }
                Container::Table(rows) => {
                    for row in rows {
                        for cell in row {
                            self.push_paragraph(cell);
                        }
                    }
                }
                Container::Inline(_) => {
                    let inlines = self.pop_sink();
                    self.push_paragraph(inlines);
                }
            }
        }

        match self.containers.pop() {
            Some(Container::Root(blocks)) => blocks,
            _ => Vec::new(),
        }
    }

    fn start_inline(&mut self, owner: InlineOwner) {
        self.containers.push(Container::Inline(owner));
        self.sinks.push(Vec::new());
    }

    fn expect_inline_marker(&mut self, owner: InlineOwner) {
        if matches!(self.containers.last(), Some(Container::Inline(_)) if self.last_owner() == Some(owner))
        {
            self.containers.pop();
            return;
        }
        // 事件流与预期不符（理论上不会发生）：退而求其次弹任意 Inline 标记
        if matches!(self.containers.last(), Some(Container::Inline(_))) {
            self.containers.pop();
        }
    }

    fn last_owner(&self) -> Option<InlineOwner> {
        match self.containers.last() {
            Some(Container::Inline(owner)) => Some(*owner),
            _ => None,
        }
    }

    fn push_wrap(&mut self, wrap: Wrap) {
        self.wraps.push(wrap);
        self.sinks.push(Vec::new());
    }

    fn pop_wrap(&mut self) {
        let inner = self.pop_sink();
        let Some(wrap) = self.wraps.pop() else {
            for inline in inner {
                self.push_inline(inline);
            }
            return;
        };
        let merged = match wrap {
            Wrap::Strong => Inline::Bold(inner),
            Wrap::Emphasis => Inline::Italic(inner),
            Wrap::Strike => Inline::Strike(inner),
            Wrap::Link { url } => Inline::Link { text: inner, url },
            Wrap::Image { url } => Inline::Image {
                alt: plain_text(&inner),
                url,
            },
        };
        self.push_inline(merged);
    }

    fn pop_sink(&mut self) -> Vec<Inline> {
        self.sinks.pop().unwrap_or_default()
    }

    fn text(&mut self, chunk: &str) {
        if chunk.is_empty() {
            return;
        }
        if let Some(Container::Code(code)) = self.containers.last_mut() {
            code.push_str(chunk);
            return;
        }
        // 与上一个 Text 合并，避免碎段
        if let Some(Inline::Text(existing)) = self.sinks.last_mut().and_then(|s| s.last_mut()) {
            existing.push_str(chunk);
            return;
        }
        self.push_inline(Inline::Text(chunk.to_owned()));
    }

    fn push_inline(&mut self, inline: Inline) {
        if self.sinks.is_empty() {
            // pulldown 在紧凑列表项内不发 Paragraph 标签（任务标记同理），
            // 这里补一个隐式收集器，由 TagEnd::Item 收尾落成段落
            self.sinks.push(Vec::new());
        }
        if let Some(sink) = self.sinks.last_mut() {
            sink.push(inline);
        }
    }

    fn push_paragraph(&mut self, inlines: Vec<Inline>) {
        if !inlines.is_empty() {
            self.push_block_raw(Block::Paragraph(inlines));
        }
    }

    /// 只有 `Inline` 标记在栈顶时，块要落到它下面的真实容器。
    fn push_block(&mut self, block: Block) {
        self.push_block_raw(block);
    }

    fn push_block_raw(&mut self, block: Block) {
        // 越过可能残留的 Inline 标记找宿主
        let mut skipped = Vec::new();
        while matches!(self.containers.last(), Some(Container::Inline(_))) {
            skipped.push(self.containers.pop());
        }
        let hosted = match self.containers.last_mut() {
            Some(Container::Root(blocks)) | Some(Container::Quote(blocks)) | Some(Container::Item(blocks)) => {
                blocks.push(block);
                true
            }
            Some(Container::List { items, .. }) => {
                // 列表直接收块（CommonMark 松散列表的裸段落）：并入最后一个条目
                if let Some(last) = items.last_mut() {
                    last.push(block);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        let _ = hosted;
        for container in skipped.into_iter().rev() {
            if let Some(container) = container {
                self.containers.push(container);
            }
        }
    }

    fn push_blocks(&mut self, blocks: Vec<Block>) {
        for block in blocks {
            self.push_block_raw(block);
        }
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            containers: vec![Container::Root(Vec::new())],
            sinks: Vec::new(),
            wraps: Vec::new(),
        }
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// 内联树的纯文本投影（图片 alt 兜底、调试断言用）。
pub fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) => out.push_str(text),
            Inline::Code(code) => out.push_str(code),
            Inline::Break => out.push(' '),
            Inline::Bold(children) | Inline::Italic(children) | Inline::Strike(children) => {
                out.push_str(&plain_text(children))
            }
            Inline::Link { text, .. } => out.push_str(&plain_text(text)),
            Inline::Image { alt, .. } => out.push_str(alt),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(inlines: &[Inline]) -> String {
        plain_text(inlines)
    }

    #[test]
    fn parses_headings_paragraphs_and_emphasis() {
        let blocks = parse("# 标题\n\n普通 **粗** *斜* ~~删~~ `码`\n");
        assert_eq!(blocks.len(), 2);
        assert!(
            matches!(&blocks[0], Block::Heading { level: 1, inlines } if text_of(inlines) == "标题")
        );
        let Block::Paragraph(inlines) = &blocks[1] else {
            panic!("期望段落: {:?}", blocks[1])
        };
        assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
        assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
        assert!(inlines.iter().any(|i| matches!(i, Inline::Strike(_))));
        assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_))));
    }

    #[test]
    fn fenced_code_keeps_raw_body() {
        let blocks = parse("```rust\nlet a = 1;\n```\n");
        assert!(matches!(&blocks[0], Block::Code(text) if text == "let a = 1;"));
    }

    #[test]
    fn quotes_lists_and_nested_lists() {
        let blocks = parse("> 引用\n\n- 甲\n- 乙\n  1. 内层\n");
        assert!(matches!(&blocks[0], Block::Quote(inner) if matches!(&inner[0], Block::Paragraph(p) if text_of(p) == "引用")));
        let Block::List {
            ordered,
            items,
            start,
        } = &blocks[1]
        else {
            panic!("期望列表: {:?}", blocks[1])
        };
        assert!(!ordered);
        assert_eq!(*start, 1);
        assert_eq!(items.len(), 2);
        assert!(matches!(&items[1][0], Block::List { ordered: true, .. }));
    }

    #[test]
    fn links_images_and_tables_flatten_sensibly() {
        let blocks = parse("[文字](https://a.b)\n\n| 甲 | 乙 |\n|----|----|\n| 1 | 2 |\n");
        let Block::Paragraph(inlines) = &blocks[0] else {
            panic!("{:?}", blocks[0])
        };
        assert!(inlines
            .iter()
            .any(|i| matches!(i, Inline::Link { url, .. } if url == "https://a.b")));
        let Block::Table(rows) = &blocks[1] else {
            panic!("{:?}", blocks[1])
        };
        assert_eq!(rows.len(), 2);
        assert_eq!(text_of(&rows[0][0]), "甲");
    }

    #[test]
    fn task_markers_and_hard_breaks() {
        let blocks = parse("- [x] 完成\n- [ ] 未完成\n");
        let Block::List { items, .. } = &blocks[0] else {
            panic!("{:?}", blocks[0])
        };
        assert_eq!(text_of_blocks(&items[0]), "[x] 完成");
        assert_eq!(text_of_blocks(&items[1]), "[ ] 未完成");
    }

    #[test]
    fn empty_and_garbage_inputs_never_panic() {
        assert!(parse("").is_empty());
        // 未闭合的粗体/代码块也要产出内容而不是吞掉
        assert!(!parse("**未闭合").is_empty());
        assert!(!parse("```\nabc").is_empty());
        assert!(!parse("~~~\n未闭合围栏").is_empty());
    }

    fn text_of_blocks(blocks: &[Block]) -> String {
        blocks
            .iter()
            .map(|b| match b {
                Block::Paragraph(inlines) | Block::Heading { inlines, .. } => plain_text(inlines),
                Block::Table(rows) => {
                    let all: Vec<Inline> = rows.iter().flatten().flatten().cloned().collect();
                    plain_text(&all)
                }
                Block::Code(text) => text.clone(),
                Block::Quote(inner) => text_of_blocks(inner),
                Block::List { items, .. } => items
                    .iter()
                    .map(|i| text_of_blocks(i))
                    .collect::<Vec<_>>()
                    .join("\n"),
                Block::Rule => "---".to_owned(),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
