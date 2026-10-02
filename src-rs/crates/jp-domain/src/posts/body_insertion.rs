/// 正文插入操作的纯函数：在指定位置插入文本，或追加到末尾。
///
/// `NEWLINE` 取 CRLF 与 .NET `Environment.NewLine`（Windows）一致；
/// 落盘前由 `PostFileFormat` 统一归一为 LF。
const NEWLINE: &str = "\r\n";

/// 在光标位置插入 markdown。
///
/// `cursor` 为 **char 索引**（非 UTF-16 码元）：WinUI TextBox 的 `SelectionStart` 是
/// UTF-16 计数，UI 层负责换算。`None` 或超出长度时追加到末尾；行中插入时前后补换行。
pub fn insert_at_cursor(body: &str, markdown: &str, cursor: Option<usize>) -> String {
    let markdown = markdown.trim_end();

    if body.is_empty() {
        return markdown.to_owned();
    }

    let chars: Vec<char> = body.chars().collect();
    let Some(index) = cursor.filter(|index| *index <= chars.len()) else {
        return append(body, markdown);
    };

    let prefix: String = chars[..index].iter().collect();
    let suffix: String = chars[index..].iter().collect();
    let need_leading = !prefix.is_empty() && !ends_with_newline(&prefix);
    let need_trailing = !suffix.is_empty() && !starts_with_newline(&suffix);

    let mut result = prefix;
    if need_leading {
        result.push_str(NEWLINE);
    }
    result.push_str(markdown);
    if need_trailing {
        result.push_str(NEWLINE);
    }
    result.push_str(&suffix);
    result
}

/// 追加到末尾（非空时以换行分隔）。
fn append(body: &str, text: &str) -> String {
    let text = text.trim_end();
    if body.is_empty() {
        text.to_owned()
    } else {
        format!("{body}{NEWLINE}{text}")
    }
}

fn ends_with_newline(text: &str) -> bool {
    matches!(text.chars().next_back(), Some('\n') | Some('\r'))
}

fn starts_with_newline(text: &str) -> bool {
    matches!(text.chars().next(), Some('\n') | Some('\r'))
}

/// 供 UI 层把 WinUI 的 UTF-16 光标位换算为 char 索引。
pub fn utf16_index_to_char_index(text: &str, utf16_index: usize) -> Option<usize> {
    let mut utf16_count = 0usize;
    for (char_index, ch) in text.chars().enumerate() {
        if utf16_count == utf16_index {
            return Some(char_index);
        }
        utf16_count += ch.len_utf16();
        if utf16_count > utf16_index {
            return None;
        }
    }
    if utf16_count == utf16_index {
        return Some(text.chars().count());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_body_returns_markdown() {
        assert_eq!(insert_at_cursor("", "![a](/x.png)\n", None), "![a](/x.png)");
    }

    #[test]
    fn null_cursor_appends_with_newline() {
        assert_eq!(insert_at_cursor("body", "extra", None), "body\r\nextra");
    }

    #[test]
    fn out_of_range_cursor_appends() {
        assert_eq!(insert_at_cursor("body", "extra", Some(99)), "body\r\nextra");
    }

    #[test]
    fn mid_line_insert_pads_newlines() {
        let body = "abcdef";

        assert_eq!(insert_at_cursor(body, "X", Some(3)), "abc\r\nX\r\ndef");
    }

    #[test]
    fn cursor_at_line_start_does_not_pad_leading_newline() {
        let body = "line1\nline2";

        assert_eq!(insert_at_cursor(body, "X", Some(6)), "line1\nX\r\nline2");
    }

    #[test]
    fn cursor_at_end_pads_only_leading_newline() {
        let body = "line1";

        assert_eq!(insert_at_cursor(body, "X", Some(5)), "line1\r\nX");
    }

    #[test]
    fn cursor_after_existing_newline_is_not_doubled() {
        let body = "a\n";

        assert_eq!(insert_at_cursor(body, "X", Some(2)), "a\nX");
    }

    #[test]
    fn chinese_text_indexes_by_char_not_byte() {
        let body = "中文正文";

        assert_eq!(insert_at_cursor(body, "X", Some(2)), "中文\r\nX\r\n正文");
    }

    #[test]
    fn utf16_index_maps_to_char_index() {
        // 中文在 BMP：每字 1 个 UTF-16 码元，索引原位对应
        assert_eq!(utf16_index_to_char_index("中文abc", 2), Some(2));
        assert_eq!(utf16_index_to_char_index("中文abc", 5), Some(5));
        // 非 BMP（emoji）占 2 个码元：码元位 2 落在 emoji 之后
        assert_eq!(utf16_index_to_char_index("😀abc", 2), Some(1));
        assert_eq!(utf16_index_to_char_index("😀abc", 4), Some(3));
        assert_eq!(utf16_index_to_char_index("abc", 3), Some(3));
        assert_eq!(utf16_index_to_char_index("abc", 4), None);
    }
}
