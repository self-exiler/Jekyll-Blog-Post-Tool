//! 博文文件格式（`---` 包裹的 YAML front matter + 正文）的唯一权威实现（ADR-007/010）：
//! 定界符切分、YAML 解析/序列化与 LF 归一在这里发生。`parse` 与 `format` 互逆，
//! 预览与落盘共用 `format`，由构造保证所见即所得。

use jp_domain::common::error::DomainResult;
use jp_domain::posts::{try_split, FrontMatter};

use crate::yaml::{parse_front_matter, serialize_front_matter};

/// 解析博文全文；无有效 front matter 块时正文即原文。
pub fn parse(content: &str) -> DomainResult<(FrontMatter, String)> {
    let (yaml, body) = try_split(content).unwrap_or_else(|| (String::new(), content.to_owned()));
    Ok((parse_front_matter(&yaml)?, body))
}

/// 序列化为最终落盘文本：front matter 块，body 非空时另起一段追加。统一 LF（ADR-010）。
pub fn format(front_matter: &FrontMatter, body: Option<&str>) -> String {
    let yaml = serialize_front_matter(front_matter);
    let mut text = format!("---\n{yaml}\n---\n");
    if let Some(body) = body.filter(|text| !text.is_empty()) {
        text.push_str(body);
    }

    normalize_new_lines(&text)
}

/// CRLF 归一为 LF。
pub fn normalize_new_lines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, NaiveDate};
    use jp_domain::posts::Category;

    use super::*;

    fn front_matter() -> FrontMatter {
        FrontMatter {
            title: "Hello World".to_owned(),
            date: Some(
                NaiveDate::from_ymd_opt(2026, 7, 28)
                    .unwrap()
                    .and_hms_opt(14, 10, 0)
                    .unwrap()
                    .and_local_timezone(FixedOffset::east_opt(8 * 3600).unwrap())
                    .single()
                    .unwrap(),
            ),
            categories: vec![Category::new("Blogging").unwrap()],
            ..FrontMatter::new()
        }
    }

    #[test]
    fn format_wraps_yaml_in_delimiters() {
        assert_eq!(
            format(&front_matter(), None),
            "---\ntitle: Hello World\ndate: 2026-07-28 14:10:00 +08:00\ncategories:\n- Blogging\n---\n"
        );
    }

    #[test]
    fn format_appends_non_empty_body_only() {
        assert!(format(&front_matter(), Some("")).ends_with("---\n"));
        assert!(format(&front_matter(), Some("正文")).ends_with("---\n正文"));
    }

    #[test]
    fn format_normalizes_crlf() {
        let text = format(&front_matter(), Some("line1\r\nline2"));

        assert!(!text.contains('\r'), "{text:?}");
        assert!(text.contains("line1\nline2"));
    }

    #[test]
    fn parse_and_format_are_inverse() {
        let content = format(&front_matter(), Some("body text"));
        let (parsed, body) = parse(&content).unwrap();

        assert_eq!(parsed.title, "Hello World");
        assert_eq!(parsed.date, front_matter().date);
        assert_eq!(parsed.categories[0].value(), "Blogging");
        assert_eq!(body, "body text");
        assert_eq!(format(&parsed, Some(&body)), content);
    }

    #[test]
    fn parse_without_front_matter_keeps_whole_text_as_body() {
        let (parsed, body) = parse("# 仅正文\n内容").unwrap();

        assert_eq!(parsed.title, "");
        assert_eq!(body, "# 仅正文\n内容");
    }

    #[test]
    fn parse_of_crlf_file_splits_delimiters_before_normalizing() {
        let (parsed, body) = parse("---\r\ntitle: T\r\n---\r\n正文").unwrap();

        assert_eq!(parsed.title, "T");
        assert_eq!(body, "正文");
    }
}
