use chrono::{FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone};
use jp_domain::common::error::{DomainError, DomainResult};
use jp_domain::posts::{Category, FrontMatter, PostDate, Tag, UnknownFields, UnknownValue};
use serde_yaml::{Mapping, Value};

/// front matter 支持的日期形态，顺序与 .NET 侧 DateFormats 一致（先精确后宽松）。
const DATE_FORMATS_WITH_OFFSET: [&str; 4] = [
    "%Y-%m-%d %H:%M:%S %:z",
    "%Y-%m-%d %H:%M:%S %z",
    "%Y-%m-%dT%H:%M:%S%:z",
    "%Y-%m-%dT%H:%M:%S%z",
];

const DATE_FORMATS_LOCAL: [&str; 4] = [
    "%Y-%m-%d %H:%M:%S",
    "%Y-%m-%dT%H:%M:%S",
    "%Y-%m-%d %H:%M",
    "%Y-%m-%d",
];

/// 将 front matter YAML 文本解析为 `FrontMatter`；未知字段保序落入 `unknown_fields`（ADR-007）。
pub fn parse_front_matter(yaml: &str) -> DomainResult<FrontMatter> {
    let mut front_matter = FrontMatter::new();

    if yaml.trim().is_empty() {
        return Ok(front_matter);
    }

    let root: Value = serde_yaml::from_str(yaml).map_err(|error| DomainError::Validation(format!(
        "front matter 解析失败：{error}"
    )))?;

    // 无根映射（仅注释、标量文档等）按空 front matter 处理
    let Value::Mapping(mapping) = root else {
        return Ok(front_matter);
    };

    let mut unknown = UnknownFields::new();

    for (key, value) in mapping.iter() {
        let Some(key) = text_of(key) else { continue };

        match key.as_str() {
            "title" => front_matter.title = text_of(value).unwrap_or_default(),
            "date" => front_matter.date = text_of(value).and_then(|text| parse_date(&text)),
            "categories" => front_matter.categories = category_list(&value)?,
            "tags" => front_matter.tags = tag_list(&value)?,
            "authors" => front_matter.authors = string_values(&value),
            // 单数字段迁移为 authors（ADR-012）
            "author" => front_matter.authors = string_values(&value),
            "description" => front_matter.description = text_of(value),
            _ => unknown.insert(&key, to_unknown(&value)),
        }
    }

    front_matter.unknown_fields = unknown;
    Ok(front_matter)
}

/// 取节点的标量文本；YAML 1.1 解析器可能把日期形态的标量挂成 tagged 值，需下钻一层。
pub fn text_of(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Null => None,
        Value::Tagged(tagged) => text_of(&tagged.value),
        _ => None,
    }
}

pub fn sequence_of(value: &Value) -> Option<Vec<Value>> {
    match value {
        Value::Sequence(items) => Some(items.clone()),
        Value::Tagged(tagged) => sequence_of(&tagged.value),
        _ => None,
    }
}

pub fn mapping_of(value: &Value) -> Option<Mapping> {
    match value {
        Value::Mapping(mapping) => Some(mapping.clone()),
        Value::Tagged(tagged) => mapping_of(&tagged.value),
        _ => None,
    }
}

fn string_values(value: &Value) -> Vec<String> {
    if let Some(items) = sequence_of(value) {
        return items
            .iter()
            .filter_map(text_of)
            .filter(|text| !text.is_empty())
            .collect();
    }

    text_of(value).filter(|text| !text.is_empty()).into_iter().collect()
}

/// categories / tags 需构造值对象；空串已被 `string_values` 滤掉，
/// 但全角空格一类"非空却非法"的项仍会被值对象拒绝，故此处保留错误向上传递。
fn category_list(value: &Value) -> DomainResult<Vec<Category>> {
    string_values(value).into_iter().map(|text| Category::new(&text)).collect()
}

fn tag_list(value: &Value) -> DomainResult<Vec<Tag>> {
    string_values(value).into_iter().map(|text| Tag::new(&text)).collect()
}

/// 未知字段转为与节点形态一一对应的递归值。
pub fn to_unknown(value: &Value) -> UnknownValue {
    match value {
        Value::Null => UnknownValue::Null,
        Value::Sequence(items) => UnknownValue::Seq(items.iter().map(to_unknown).collect()),
        Value::Mapping(mapping) => UnknownValue::Map(
            mapping
                .iter()
                .filter_map(|(key, value)| text_of(key).map(|key| (key, to_unknown(value))))
                .collect(),
        ),
        Value::Tagged(tagged) => to_unknown(&tagged.value),
        other => text_of(other).map(UnknownValue::Scalar).unwrap_or(UnknownValue::Null),
    }
}

fn parse_date(text: &str) -> Option<PostDate> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    for format in DATE_FORMATS_WITH_OFFSET {
        if let Ok(parsed) = chrono::DateTime::parse_from_str(text, format) {
            return Some(parsed);
        }
    }

    for format in DATE_FORMATS_LOCAL {
        let naive = if format.ends_with("%Y-%m-%d") {
            NaiveDate::parse_from_str(text, "%Y-%m-%d")
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
        } else {
            NaiveDateTime::parse_from_str(text, format).ok()
        };

        if let Some(naive) = naive {
            // 无偏移量的日期时间按本地时区解读（与 .NET DateTimeOffset.TryParse 行为一致）
            return localize(&naive);
        }
    }

    None
}

fn localize(naive: &NaiveDateTime) -> Option<PostDate> {
    let local = Local.from_local_datetime(naive);
    // 时区重叠取较早一侧；时区缝隙（该本地时间不存在）退化为零偏移（UTC），
    // 保证 date 仍可解析，而不是整条 front matter 丢日期
    local
        .earliest()
        .map(|value| value.fixed_offset())
        .or_else(|| {
            FixedOffset::east_opt(0).and_then(|offset| naive.and_local_timezone(offset).single())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn parses_all_known_fields() {
        let yaml = "title: Hello World\ndate: 2026-07-28 14:10:00 +08:00\ncategories:\n  - Blogging\n  - Tech\ntags:\n  - Jekyll\n  - WinUI\nauthors:\n  - cotes\ndescription: A test post";

        let front_matter = parse_front_matter(yaml).unwrap();

        assert_eq!(front_matter.title, "Hello World");
        let date = front_matter.date.unwrap();
        assert_eq!((date.year(), date.month(), date.day()), (2026, 7, 28));
        assert_eq!(date.offset().local_minus_utc(), 8 * 3600);
        assert_eq!(front_matter.categories.len(), 2);
        assert_eq!(front_matter.categories[0].value(), "Blogging");
        assert_eq!(front_matter.tags.len(), 2);
        assert_eq!(front_matter.tags[1].value(), "WinUI");
        assert_eq!(front_matter.authors, vec!["cotes"]);
        assert_eq!(front_matter.description.as_deref(), Some("A test post"));
        assert!(front_matter.unknown_fields.is_empty());
    }

    #[test]
    fn flow_style_sequence_is_accepted() {
        let front_matter = parse_front_matter("title: T\ntags: [a, b]").unwrap();

        assert_eq!(front_matter.tags.len(), 2);
    }

    #[test]
    fn empty_yaml_yields_empty_front_matter() {
        for yaml in ["", "   \n  \n"] {
            let front_matter = parse_front_matter(yaml).unwrap();

            assert_eq!(front_matter.title, "");
            assert!(front_matter.date.is_none());
            assert!(front_matter.categories.is_empty());
            assert!(front_matter.unknown_fields.is_empty());
        }
    }

    #[test]
    fn comment_only_yaml_yields_empty_front_matter() {
        assert!(parse_front_matter("# 仅注释\n").unwrap().title.is_empty());
    }

    #[test]
    fn scalar_category_is_promoted_to_single_item_list() {
        let front_matter = parse_front_matter("title: Test\ncategories: Blogging").unwrap();

        assert_eq!(front_matter.categories.len(), 1);
        assert_eq!(front_matter.categories[0].value(), "Blogging");
    }

    #[test]
    fn singular_author_migrates_to_authors() {
        let front_matter = parse_front_matter("title: Test\nauthor: cotes").unwrap();

        assert_eq!(front_matter.authors, vec!["cotes"]);
    }

    #[test]
    fn parses_date_shapes_with_and_without_offset() {
        for (text, expect_offset) in [
            ("2026-07-28", None),
            ("2026-07-28 14:10:00", None),
            ("2026-07-28 14:10:00 +08:00", Some(8 * 3600)),
            ("2026-07-28T14:10:00+08:00", Some(8 * 3600)),
            ("2026-07-28 14:10:00 -0530", Some(-(5 * 3600 + 30 * 60))),
        ] {
            let front_matter = parse_front_matter(&format!("date: {text}")).unwrap();
            let date = front_matter.date.unwrap_or_else(|| panic!("无法解析 {text}"));

            assert_eq!((date.year(), date.month(), date.day()), (2026, 7, 28), "text: {text}");
            if let Some(offset) = expect_offset {
                assert_eq!(date.offset().local_minus_utc(), offset, "text: {text}");
            }
        }
    }

    #[test]
    fn unparseable_date_is_dropped_not_fatal() {
        let front_matter = parse_front_matter("title: T\ndate: 昨天下午").unwrap();

        assert!(front_matter.date.is_none());
        assert_eq!(front_matter.title, "T");
    }

    #[test]
    fn unknown_fields_keep_scalars_lists_and_nested_maps() {
        let yaml = "title: Test\ncustom_field: value\nanother:\n  - item1\n  - item2\nmath:\n  enable: true\n  katex: false";

        let front_matter = parse_front_matter(yaml).unwrap();

        assert_eq!(front_matter.unknown_fields.keys(), vec!["custom_field", "another", "math"]);
        assert_eq!(
            front_matter.unknown_fields.get("custom_field").and_then(UnknownValue::as_scalar),
            Some("value")
        );
        assert!(matches!(
            front_matter.unknown_fields.get("another"),
            Some(UnknownValue::Seq(items)) if items.len() == 2
        ));
        assert!(matches!(
            front_matter.unknown_fields.get("math"),
            Some(UnknownValue::Map(entries)) if entries.len() == 2
        ));
    }

    #[test]
    fn missing_description_stays_none() {
        let front_matter = parse_front_matter("title: Test").unwrap();

        assert_eq!(front_matter.description, None);
    }

    #[test]
    fn malformed_yaml_reports_error() {
        let error = parse_front_matter("title: [\nunclosed").unwrap_err();

        assert!(matches!(error, DomainError::Validation(_)));
    }

    #[test]
    fn whitespace_category_is_rejected_as_error() {
        // 空串在采集阶段就被过滤，只剩"非空但全空白"的项会走到值对象校验
        let error = parse_front_matter("categories:\n- \" \"\n- ok").unwrap_err();

        assert!(matches!(error, DomainError::Validation(_)));
    }

    #[test]
    fn blank_category_items_are_dropped() {
        let front_matter = parse_front_matter("categories:\n- \"\"\n- ok").unwrap();

        assert_eq!(front_matter.categories.len(), 1);
        assert_eq!(front_matter.categories[0].value(), "ok");
    }
}
