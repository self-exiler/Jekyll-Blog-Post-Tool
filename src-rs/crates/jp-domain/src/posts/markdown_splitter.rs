/// 将 markdown 文档切分为 front matter YAML 与正文段。
///
/// 仅在行首匹配 `---` 作为定界符，避免正文中含 `---`（如水平分割线）时误切分。

const DELIMITER: &str = "---";

/// 定界符行的空白裁剪：与 .NET `Trim('\r', ' ', '\t')` 一致。
fn is_delimiter_padding(ch: char) -> bool {
    matches!(ch, '\r' | ' ' | '\t')
}

/// 尝试切分；`None` 表示无有效 front matter 块。
pub fn try_split(content: &str) -> Option<(String, String)> {
    if content.is_empty() {
        return None;
    }

    let lines: Vec<&str> = content.split('\n').collect();

    // 开定界符：跳过前导空行，第一个非空行必须是 "---"
    let open_index = lines
        .iter()
        .position(|line| !line.trim_matches(is_delimiter_padding).is_empty())
        .filter(|index| lines[*index].trim_matches(is_delimiter_padding) == DELIMITER)?;

    // 闭定界符：后续第一个独占一行的 "---"
    let close_index = ((open_index + 1)..lines.len()).find(|index| {
        lines[*index].trim_matches(is_delimiter_padding) == DELIMITER
    })?;

    let yaml = join_lines(&lines, open_index + 1, close_index).trim().to_owned();
    let body = join_lines(&lines, close_index + 1, lines.len())
        .trim_start_matches(['\r', '\n'])
        .to_owned();

    Some((yaml, body))
}

fn join_lines(lines: &[&str], start: usize, end: usize) -> String {
    if start >= end {
        return String::new();
    }
    lines[start..end].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_front_matter_splits_correctly() {
        let content = "---\ntitle: Hello\ndate: 2026-07-28\n---\nThis is the body.";

        let (yaml, body) = try_split(content).unwrap();

        assert!(yaml.contains("title: Hello"));
        assert!(yaml.contains("date: 2026-07-28"));
        assert_eq!(body, "This is the body.");
    }

    #[test]
    fn no_front_matter_returns_none() {
        let content = "Just some text without front matter.";

        assert!(try_split(content).is_none());
    }

    #[test]
    fn only_opening_delimiter_returns_none() {
        assert!(try_split("---\ntitle: Hello\nNo closing delimiter").is_none());
    }

    #[test]
    fn empty_body_yields_empty_string() {
        let (yaml, body) = try_split("---\ntitle: Hello\n---\n").unwrap();

        assert_eq!(yaml, "title: Hello");
        assert_eq!(body, "");
    }

    #[test]
    fn leading_blank_lines_are_skipped() {
        let (yaml, body) = try_split("\n\n---\ntitle: Hello\n---\nBody here.").unwrap();

        assert_eq!(yaml, "title: Hello");
        assert_eq!(body, "Body here.");
    }

    #[test]
    fn horizontal_rule_in_body_does_not_split() {
        let content = "Just body.\n\n---\n\nMore body.";

        assert!(try_split(content).is_none());
    }

    #[test]
    fn crlf_content_is_split_and_body_trimmed() {
        let (yaml, body) = try_split("---\r\ntitle: Hello\r\n---\r\n\r\nBody.").unwrap();

        assert_eq!(yaml, "title: Hello");
        assert_eq!(body, "Body.");
    }

    #[test]
    fn delimiter_with_trailing_spaces_is_recognised() {
        let (yaml, body) = try_split("--- \ntitle: Hello\n ---\nBody").unwrap();

        assert_eq!(yaml, "title: Hello");
        assert_eq!(body, "Body");
    }

    #[test]
    fn empty_content_returns_none() {
        assert!(try_split("").is_none());
    }
}
