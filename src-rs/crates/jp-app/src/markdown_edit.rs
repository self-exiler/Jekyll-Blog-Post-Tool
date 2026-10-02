//! Markdown 编辑动作（对偶 `PostBodyPage.xaml.cs` 的编辑辅助方法）。
//!
//! 全部为纯函数：输入「当前文本 + 选区」，输出「新文本 + 新选区」，不碰任何控件，
//! 因此可单元测试。索引一律按 **UTF-16 代码单元** 计数——`TextBox.SelectionStart`
//! 与 `SelectionLength` 就是这个口径，按 `char` 或字节数走会和控件错位。

use windows_core::HSTRING;

pub const TABLE_SNIPPET: &str = "| 列1 | 列2 | 列3 |";
pub const TABLE_DIVIDER: &str = "| --- | --- | --- |";
pub const TABLE_ROW: &str = "|  |  |  |";

/// 与 .NET `Environment.NewLine` 在 Windows 上一致。
pub const NEWLINE: &str = "\r\n";
const NEWLINE_UNITS: usize = 2;
const LINE_FEED: u16 = b'\n' as u16;

/// 文本快照 + 选区（`start`/`length` 为 UTF-16 单元数）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub text: String,
    pub start: usize,
    pub length: usize,
}

impl Selection {
    /// 从控件取值。越界（控件与状态短暂不一致时会出现）按夹紧处理。
    pub fn new(text: &str, start: i32, length: i32) -> Self {
        let total = utf16_len(text);
        let start = (start.max(0) as usize).min(total);
        let length = (length.max(0) as usize).min(total - start);
        Self {
            text: text.to_owned(),
            start,
            length,
        }
    }

    pub fn end(&self) -> usize {
        self.start + self.length
    }

    pub fn selected_text(&self) -> String {
        String::from_utf16_lossy(&self.units()[self.start..self.end()])
    }

    fn units(&self) -> Vec<u16> {
        self.text.encode_utf16().collect()
    }
}

/// 一次编辑的结果：直接写回控件即可（`Text` + 选区）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub text: String,
    pub selection_start: usize,
    pub selection_length: usize,
}

impl Edit {
    pub fn text_hstring(&self) -> HSTRING {
        HSTRING::from(self.text.as_str())
    }

    fn from_units(units: &[u16], start: usize, length: usize) -> Self {
        Self {
            text: String::from_utf16_lossy(units),
            selection_start: start,
            selection_length: length,
        }
    }
}

pub fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn units_of(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn replace(units: &mut Vec<u16>, at: usize, len: usize, with: &str) {
    let replacement = units_of(with);
    units.splice(at..at + len, replacement);
}

fn starts_with_at(units: &[u16], at: usize, marker: &str) -> bool {
    let marker = units_of(marker);
    at <= units.len() && units.len() - at >= marker.len() && units[at..at + marker.len()] == marker[..]
}

/// 用 `marker` 包裹选区；选区两侧已有同一标记时解除包裹；无选区时插入成对标记并把光标居中。
pub fn wrap(marker: &str, selection: &Selection) -> Edit {
    let marker_units = utf16_len(marker);
    let mut units = selection.units();
    let start = selection.start;
    let end = selection.end();

    if start >= marker_units
        && end + marker_units <= units.len()
        && starts_with_at(&units, start - marker_units, marker)
        && starts_with_at(&units, end, marker)
    {
        replace(&mut units, end, marker_units, "");
        replace(&mut units, start - marker_units, marker_units, "");
        return Edit::from_units(&units, start - marker_units, selection.length);
    }

    if selection.length > 0 {
        replace(&mut units, end, 0, marker);
        replace(&mut units, start, 0, marker);
        return Edit::from_units(&units, start + marker_units, selection.length);
    }

    let paired = format!("{marker}{marker}");
    replace(&mut units, start, 0, &paired);
    Edit::from_units(&units, start + marker_units, 0)
}

/// 给选区覆盖的每一行加行首前缀；首行已有前缀时改为移除。
pub fn line_prefix(prefix: &str, selection: &Selection) -> Edit {
    let mut units = selection.units();
    let start = selection.start;
    let end = selection.end();

    let line_starts = collect_line_starts(&units, start, end);
    let Some(&first_line) = line_starts.first() else {
        return Edit::from_units(&units, start, selection.length);
    };

    let removing = starts_with_at(&units, first_line, prefix);
    let prefix_units = utf16_len(prefix);
    let mut start_delta = 0isize;
    let mut end_delta = 0isize;

    // 从最后一行往前改，前面的偏移不受影响
    for at in line_starts.iter().rev().copied() {
        if removing && !starts_with_at(&units, at, prefix) {
            continue;
        }

        if removing {
            replace(&mut units, at, prefix_units, "");
        } else {
            replace(&mut units, at, 0, prefix);
        }

        // 行首在选区起止位置之前的编辑才影响对应位置
        if at <= start {
            start_delta += if removing { -(prefix_units as isize) } else { prefix_units as isize };
        }
        if at <= end {
            end_delta += if removing { -(prefix_units as isize) } else { prefix_units as isize };
        }
    }

    let new_start = (start as isize + start_delta).max(0) as usize;
    let new_end = (end as isize + end_delta).max(0) as usize;
    Edit::from_units(&units, new_start, new_end.saturating_sub(new_start))
}

/// 围栏代码块：包住选区（或插入空代码块），光标移到围栏内容首。
pub fn code_block(selection: &Selection) -> Edit {
    const FENCE: &str = "```";
    let fence_units = utf16_len(FENCE);

    let selected = selection.selected_text().trim_end_matches(['\r', '\n']).to_owned();
    let inner = if selected.is_empty() {
        String::new()
    } else {
        format!("{selected}{NEWLINE}")
    };
    let snippet = format!("{FENCE}{NEWLINE}{inner}{FENCE}");

    let mut units = selection.units();
    replace(&mut units, selection.start, selection.length, &snippet);
    Edit::from_units(&units, selection.start + fence_units + NEWLINE_UNITS, 0)
}

/// 插入链接：选中 http/https URL 时把选中内容放进括号并选中链接文本，
/// 否则把选中内容（或占位符）放进方括号并选中 `url`。
pub fn link(selection: &Selection) -> Edit {
    let selected = selection.selected_text();
    let is_http_url = selection.length > 0
        && (selected.starts_with("http://") || selected.starts_with("https://"));

    let (insert, offset, length) = if is_http_url {
        (format!("[链接文本]({selected})"), 1usize, 4usize)
    } else {
        let label = if selection.length > 0 {
            selected
        } else {
            "链接文本".to_owned()
        };
        (
            format!("[{label}](url)"),
            utf16_len(&label) + 3,
            3,
        )
    };

    let mut units = selection.units();
    replace(&mut units, selection.start, selection.length, &insert);
    Edit::from_units(&units, selection.start + offset, length)
}

/// 表格片段（三列）。
pub fn table(selection: &Selection) -> Edit {
    insert_snippet(
        &format!("{TABLE_SNIPPET}{NEWLINE}{TABLE_DIVIDER}{NEWLINE}{TABLE_ROW}"),
        selection,
    )
}

/// 在光标处插入片段（不在行首时先补换行，光标移到片段之后）。
pub fn insert_snippet(snippet: &str, selection: &Selection) -> Edit {
    let mut units = selection.units();
    let at_line_start = selection.start == 0 || units[selection.start - 1] == LINE_FEED;
    let prefix = if at_line_start { "" } else { NEWLINE };
    let insert = format!("{prefix}{snippet}{NEWLINE}");

    replace(&mut units, selection.start, selection.length, &insert);
    Edit::from_units(&units, selection.start + utf16_len(&insert), 0)
}

/// 收集光标/选区覆盖到的行首（UTF-16 下标）；选区以换行结尾时不包含下一行。
fn collect_line_starts(units: &[u16], start: usize, sel_end: usize) -> Vec<usize> {
    let mut line_starts = Vec::new();
    let mut line_start = if start == 0 {
        0
    } else {
        units[..start]
            .iter()
            .rposition(|&unit| unit == LINE_FEED)
            .map_or(0, |index| index + 1)
    };

    while line_start <= sel_end {
        if line_start < sel_end || start == sel_end {
            line_starts.push(line_start);
        }

        match units[line_start..].iter().position(|&unit| unit == LINE_FEED) {
            Some(offset) if line_start + offset < sel_end => line_start += offset + 1,
            _ => break,
        }
    }

    line_starts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(text: &str, start: i32, length: i32) -> Selection {
        Selection::new(text, start, length)
    }

    #[test]
    fn wraps_selection() {
        let edit = wrap("**", &sel("hello", 0, 5));
        assert_eq!(edit.text, "**hello**");
        assert_eq!((edit.selection_start, edit.selection_length), (2, 5));
    }

    #[test]
    fn unwraps_when_markers_already_present() {
        let edit = wrap("**", &sel("**hello**", 2, 5));
        assert_eq!(edit.text, "hello");
        assert_eq!((edit.selection_start, edit.selection_length), (0, 5));
    }

    #[test]
    fn empty_selection_inserts_pair_and_places_caret_inside() {
        let edit = wrap("`", &sel("a b", 1, 0));
        assert_eq!(edit.text, "a`` b");
        assert_eq!(edit.selection_start, 2);
    }

    #[test]
    fn indices_count_utf16_units_not_chars_or_bytes() {
        // 单个汉字：UTF-8 三字节、UTF-16 一个单元
        let selection = sel("中文abc", 2, 3);
        assert_eq!(selection.selected_text(), "abc");

        let edit = wrap("**", &selection);
        assert_eq!(edit.text, "中文**abc**");
        assert_eq!(edit.selection_start, 4);
    }

    #[test]
    fn surrogate_pairs_count_as_two_units() {
        let selection = sel("😀x", 2, 1);
        assert_eq!(utf16_len("😀x"), 3);
        assert_eq!(selection.selected_text(), "x");
    }

    #[test]
    fn prefix_applies_to_every_covered_line() {
        let edit = line_prefix("- ", &sel("a\nb\nc", 0, 5));
        assert_eq!(edit.text, "- a\n- b\n- c");
        assert_eq!((edit.selection_start, edit.selection_length), (2, 9));
    }

    #[test]
    fn prefix_removed_when_first_line_has_it() {
        let edit = line_prefix("- ", &sel("- a\n- b", 0, 7));
        assert_eq!(edit.text, "a\nb");
        assert_eq!((edit.selection_start, edit.selection_length), (0, 3));
    }

    #[test]
    fn caret_only_prefix_acts_on_current_line() {
        let edit = line_prefix("> ", &sel("a\nbb", 3, 0));
        assert_eq!(edit.text, "a\n> bb");
        assert_eq!(edit.selection_start, 5);
    }

    #[test]
    fn selection_ending_at_newline_excludes_next_line() {
        let edit = line_prefix("# ", &sel("a\nb\n", 0, 4));
        assert_eq!(edit.text, "# a\n# b\n");
    }

    #[test]
    fn code_block_wraps_selection_and_places_caret_after_fence() {
        let edit = code_block(&sel("x = 1", 0, 5));
        assert_eq!(edit.text, "```\r\nx = 1\r\n```");
        assert_eq!(edit.selection_start, 5);
    }

    #[test]
    fn code_block_without_selection_inserts_empty_fence() {
        let edit = code_block(&sel("", 0, 0));
        assert_eq!(edit.text, "```\r\n```");
    }

    #[test]
    fn link_with_selected_url_selects_label() {
        let edit = link(&sel("https://a.test", 0, 14));
        assert_eq!(edit.text, "[链接文本](https://a.test)");
        assert_eq!((edit.selection_start, edit.selection_length), (1, 4));
    }

    #[test]
    fn link_without_selection_selects_url_placeholder() {
        let edit = link(&sel("", 0, 0));
        assert_eq!(edit.text, "[链接文本](url)");
        assert_eq!((edit.selection_start, edit.selection_length), (7, 3));
    }

    #[test]
    fn table_snippet_starts_on_its_own_line() {
        let edit = table(&sel("text", 4, 0));
        assert!(edit.text.starts_with("text\r\n| 列1 | 列2 | 列3 |"));
        assert_eq!(edit.selection_start, utf16_len(&edit.text));
    }

    #[test]
    fn snippet_at_line_start_is_not_prefixed_with_newline() {
        let edit = insert_snippet("x", &sel("", 0, 0));
        assert_eq!(edit.text, "x\r\n");
    }

    #[test]
    fn out_of_range_selection_is_clamped() {
        let selection = sel("abc", 99, 99);
        assert_eq!((selection.start, selection.length), (3, 0));
    }
}
