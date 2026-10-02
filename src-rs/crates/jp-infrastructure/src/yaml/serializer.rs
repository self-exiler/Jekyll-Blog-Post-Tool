//! front matter → YAML 文本。字段顺序与 .NET `YamlFrontMatterSerializer` 一致：
//! title / date / categories / tags / authors / description，其后按磁盘书写顺序追加未知字段（ADR-007）。

use jp_domain::posts::{FrontMatter, UnknownValue};

use super::emitter::write_map;

/// .NET 侧 `"yyyy-MM-dd HH:mm:ss zzz"` 的等价格式（偏移量带冒号）。
const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S %:z";

pub fn serialize_front_matter(front_matter: &FrontMatter) -> String {
    let mut entries: Vec<(String, UnknownValue)> = Vec::new();

    entries.push(("title".to_owned(), UnknownValue::scalar(front_matter.title.clone())));

    if let Some(date) = front_matter.date {
        entries.push(("date".to_owned(), UnknownValue::scalar(date.format(DATE_FORMAT).to_string())));
    }

    if !front_matter.categories.is_empty() {
        entries.push((
            "categories".to_owned(),
            UnknownValue::seq(front_matter.categories.iter().map(|c| UnknownValue::scalar(c.value()))),
        ));
    }

    if !front_matter.tags.is_empty() {
        entries.push((
            "tags".to_owned(),
            UnknownValue::seq(front_matter.tags.iter().map(|t| UnknownValue::scalar(t.value()))),
        ));
    }

    if !front_matter.authors.is_empty() {
        entries.push((
            "authors".to_owned(),
            UnknownValue::seq(front_matter.authors.iter().map(|a| UnknownValue::scalar(a.clone()))),
        ));
    }

    // description 为空白时整字段省略，避免 Chirpy 渲染出空的副标题行
    if let Some(description) = front_matter.description.as_deref().filter(|text| !text.trim().is_empty()) {
        entries.push(("description".to_owned(), UnknownValue::scalar(description)));
    }

    for (key, value) in front_matter.unknown_fields.iter() {
        entries.push((key.clone(), value.clone()));
    }

    let mut out = String::new();
    write_map(&mut out, &entries, 0);
    out.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use crate::yaml::parse_front_matter;
    use chrono::FixedOffset;
    use jp_domain::posts::{Category, Tag};

    use super::*;

    fn date() -> Option<jp_domain::posts::PostDate> {
        chrono::NaiveDate::from_ymd_opt(2026, 7, 28)?
            .and_hms_opt(14, 10, 0)?
            .and_local_timezone(FixedOffset::east_opt(8 * 3600)?)
            .single()
    }

    fn populated() -> FrontMatter {
        FrontMatter {
            title: "Hello World".to_owned(),
            date: date(),
            categories: vec![Category::new("Blogging").unwrap(), Category::new("Tech").unwrap()],
            tags: vec![Tag::new("Jekyll").unwrap(), Tag::new("WinUI").unwrap()],
            authors: vec!["cotes".to_owned()],
            description: Some("A test post".to_owned()),
            ..FrontMatter::new()
        }
    }

    #[test]
    fn serialize_all_fields_in_known_order() {
        let yaml = serialize_front_matter(&populated());

        assert!(yaml.contains("title: Hello World"), "{yaml}");
        assert!(yaml.contains("date: 2026-07-28 14:10:00 +08:00"), "{yaml}");
        assert!(yaml.contains("categories:\n- Blogging\n- Tech\n"), "{yaml}");
        assert!(yaml.contains("tags:\n- Jekyll\n- WinUI\n"), "{yaml}");
        assert!(yaml.contains("authors:\n- cotes\n"), "{yaml}");
        assert!(yaml.contains("description: A test post"), "{yaml}");
        assert!(!yaml.ends_with('\n'), "trailing newline must be trimmed: {yaml:?}");
    }

    #[test]
    fn serialize_minimal_front_matter() {
        let fm = FrontMatter {
            title: "Test".to_owned(),
            date: Some(
                chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(FixedOffset::east_opt(0).unwrap())
                    .single()
                    .unwrap(),
            ),
            ..FrontMatter::new()
        };

        let yaml = serialize_front_matter(&fm);

        assert_eq!(yaml, "title: Test\ndate: 2026-01-01 00:00:00 +00:00");
        assert!(!yaml.contains("categories:"));
        assert!(!yaml.contains("tags:"));
        assert!(!yaml.contains("authors:"));
        assert!(!yaml.contains("description:"));
    }

    #[test]
    fn serialize_omits_missing_date_and_blank_description() {
        let yaml = serialize_front_matter(&FrontMatter {
            title: "Test".to_owned(),
            description: Some("   ".to_owned()),
            ..FrontMatter::new()
        });

        assert!(!yaml.contains("date:"));
        assert!(!yaml.contains("description:"));
    }

    #[test]
    fn serialize_appends_unknown_fields_after_known_ones() {
        let mut fm = populated();
        fm.unknown_fields.insert("custom", UnknownValue::scalar("value"));
        fm.unknown_fields
            .insert("legacy", UnknownValue::seq([UnknownValue::scalar("a"), UnknownValue::scalar("b")]));

        let yaml = serialize_front_matter(&fm);

        let description = yaml.find("description:").expect("description");
        let custom = yaml.find("custom: value").expect("custom");
        let legacy = yaml.find("legacy:").expect("legacy");
        assert!(description < custom && custom < legacy, "unexpected field order:\n{yaml}");
        assert!(yaml.contains("- a") && yaml.contains("- b"), "{yaml}");
    }

    #[test]
    fn round_trip_preserves_known_and_unknown_fields() {
        let mut fm = populated();
        fm.unknown_fields.insert("custom_field", UnknownValue::scalar("custom_value"));
        fm.unknown_fields.insert("math", UnknownValue::Map(vec![("enable".to_owned(), UnknownValue::scalar("true"))]));

        let parsed = parse_front_matter(&serialize_front_matter(&fm)).unwrap();

        assert_eq!(parsed.title, fm.title);
        assert_eq!(parsed.date, fm.date);
        assert_eq!(parsed.categories.len(), 2);
        assert_eq!(parsed.tags.len(), 2);
        assert_eq!(parsed.authors, fm.authors);
        assert_eq!(parsed.description.as_deref(), Some("A test post"));
        assert_eq!(parsed.unknown_fields.keys(), vec!["custom_field", "math"]);
        assert_eq!(
            parsed.unknown_fields.get("custom_field").and_then(UnknownValue::as_scalar),
            Some("custom_value")
        );
    }
}
