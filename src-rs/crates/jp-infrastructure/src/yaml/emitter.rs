//! 手写 YAML 块式输出器。
//!
//! 之所以不用 YAML 库的序列化器：本工具的预览与落盘共用同一份文本（ADR-007 所见即所得），
//! 输出必须逐字节稳定 —— 已知字段恒定 plain 风格（`date: 2026-07-28 14:10:00 +08:00` 不带引号，
//! 与 YamlDotNet 一致），序列不额外缩进（`key:` 后接顶格 `- item`）。

use jp_domain::posts::UnknownValue;

/// 把键或标量渲染为 YAML 文本片段。
pub fn scalar(text: &str) -> String {
    if plain_is_safe(text) {
        text.to_owned()
    } else {
        quoted(text)
    }
}

/// 写出映射：`indent` 个前导空格 + `key: value`。
pub fn write_map(out: &mut String, entries: &[(String, UnknownValue)], indent: usize) {
    for (key, value) in entries {
        push_indent(out, indent);
        out.push_str(&scalar(key));
        out.push(':');
        write_value(out, value, indent);
    }
}

/// 写出前缀已就位（调用方写好缩进与 `key:`）之后的值部分，总以换行收尾。
pub fn write_value(out: &mut String, value: &UnknownValue, indent: usize) {
    match value {
        UnknownValue::Null => out.push('\n'),
        UnknownValue::Scalar(text) => {
            out.push(' ');
            out.push_str(&scalar(text));
            out.push('\n');
        }
        UnknownValue::Seq(items) => {
            if items.is_empty() {
                out.push_str(" []\n");
                return;
            }
            out.push('\n');
            write_seq(out, items, indent);
        }
        UnknownValue::Map(entries) => {
            if entries.is_empty() {
                out.push_str(" {}\n");
                return;
            }
            out.push('\n');
            write_map(out, entries, indent + 2);
        }
    }
}

fn write_seq(out: &mut String, items: &[UnknownValue], indent: usize) {
    for item in items {
        push_indent(out, indent);
        out.push('-');
        match item {
            UnknownValue::Null => out.push('\n'),
            UnknownValue::Scalar(text) => {
                out.push(' ');
                out.push_str(&scalar(text));
                out.push('\n');
            }
            UnknownValue::Seq(nested) => {
                out.push('\n');
                write_seq(out, nested, indent + 2);
            }
            UnknownValue::Map(entries) => {
                out.push(' ');
                write_map_first_key_inline(out, entries, indent + 2);
            }
        }
    }
}

/// 序列项内的映射：首个键接在 `- ` 之后，其余键按 `indent` 对齐。
fn write_map_first_key_inline(out: &mut String, entries: &[(String, UnknownValue)], indent: usize) {
    for (index, (key, value)) in entries.iter().enumerate() {
        if index > 0 {
            push_indent(out, indent);
        }
        out.push_str(&scalar(key));
        out.push(':');
        write_value(out, value, indent);
    }
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}

/// plain 形态是否安全：不空、首尾无空白、不含会被 YAML 误读的语法。
fn plain_is_safe(text: &str) -> bool {
    if text.is_empty() || text.trim() != text {
        return false;
    }

    if text
        .chars()
        .any(|ch| ch == '\n' || ch == '\r' || ch == '\t' || (ch as u32) < 0x20)
    {
        return false;
    }

    if text.contains(": ") || text.contains(" #") || text.ends_with(':') {
        return false;
    }

    let mut chars = text.chars();
    let first = chars.next().expect("非空串必有首字符");
    if matches!(first, '#' | ',' | '[' | ']' | '{' | '}' | '&' | '*' | '!' | '|' | '>' | '\'' | '"' | '%' | '@' | '`') {
        return false;
    }

    // `- ? :` 仅在后随空格（或单独成串）时是指示符
    if matches!(first, '-' | '?' | ':') && chars.next().is_none_or(|next| next == ' ') {
        return false;
    }

    true
}

fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            // 双引号内的裸换行会被解析器折叠成空格：多行描述必须走转义序列
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{7f}'..='\u{9f}' => out.push_str(&format!("\\x{:02X}", ch as u32)),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, value: UnknownValue) -> (String, UnknownValue) {
        (key.to_owned(), value)
    }

    fn render(entries: Vec<(String, UnknownValue)>) -> String {
        let mut out = String::new();
        write_map(&mut out, &entries, 0);
        out
    }

    #[test]
    fn scalars_are_plain_when_safe() {
        let yaml = render(vec![
            entry("title", UnknownValue::scalar("Hello World")),
            entry("date", UnknownValue::scalar("2026-07-28 14:10:00 +08:00")),
            entry("pin", UnknownValue::scalar("true")),
        ]);

        assert_eq!(
            yaml,
            "title: Hello World\ndate: 2026-07-28 14:10:00 +08:00\npin: true\n"
        );
    }

    #[test]
    fn chinese_scalars_stay_plain() {
        let yaml = render(vec![entry("title", UnknownValue::scalar("用 Rust 重写博客工具"))]);

        assert_eq!(yaml, "title: 用 Rust 重写博客工具\n");
    }

    #[test]
    fn ambiguous_scalars_are_quoted() {
        for (input, expected) in [
            ("a: b", "\"a: b\""),
            ("#note", "\"#note\""),
            ("- item", "\"- item\""),
            ("", "\"\""),
            (" padded ", "\" padded \""),
            ("ends with colon:", "\"ends with colon:\""),
        ] {
            assert_eq!(scalar(input), expected, "input: {input}");
        }
    }

    #[test]
    fn quotes_and_backslashes_are_escaped() {
        // 引号不在首位、反斜杠本身都不破坏 plain，与 YamlDotNet 一致保持裸写
        assert_eq!(scalar("say \"hi\""), "say \"hi\"");
        assert_eq!(scalar("C:\\path"), "C:\\path");
        // 首位是引号必须转义；换行在双引号内会被折叠，必须走 \n 序列
        assert_eq!(scalar("\"quoted\""), "\"\\\"quoted\\\"\"");
        assert_eq!(scalar("a\nb"), "\"a\\nb\"");
        assert_eq!(scalar("a\tb"), "\"a\\tb\"");
    }

    #[test]
    fn sequences_are_not_indented_under_key() {
        let yaml = render(vec![entry(
            "categories",
            UnknownValue::seq([UnknownValue::scalar("Blogging"), UnknownValue::scalar("Tech")]),
        )]);

        assert_eq!(yaml, "categories:\n- Blogging\n- Tech\n");
    }

    #[test]
    fn empty_collections_use_flow_style() {
        let yaml = render(vec![
            entry("tags", UnknownValue::seq([])),
            entry("meta", UnknownValue::Map(Vec::new())),
        ]);

        assert_eq!(yaml, "tags: []\nmeta: {}\n");
    }

    #[test]
    fn nested_mapping_indents_by_two() {
        // 冒号后不跟空格的标量（date、data URI）必须保持 plain，与 YamlDotNet 输出一致
        let yaml = render(vec![entry(
            "image",
            UnknownValue::Map(vec![
                entry("path", UnknownValue::scalar("/assets/img/a.png")),
                entry("lqip", UnknownValue::scalar("data:image/gif;base64,R0")),
            ]),
        )]);

        assert_eq!(
            yaml,
            "image:\n  path: /assets/img/a.png\n  lqip: data:image/gif;base64,R0\n"
        );
    }

    #[test]
    fn mapping_inside_sequence_puts_first_key_inline() {
        let yaml = render(vec![entry(
            "prompts",
            UnknownValue::seq([UnknownValue::Map(vec![
                entry("name", UnknownValue::scalar("overview")),
                entry("link", UnknownValue::scalar("/docs")),
            ])]),
        )]);

        assert_eq!(yaml, "prompts:\n- name: overview\n  link: /docs\n");
    }

    #[test]
    fn null_value_writes_bare_key() {
        let yaml = render(vec![entry("mermaid", UnknownValue::Null)]);

        assert_eq!(yaml, "mermaid:\n");
    }
}
