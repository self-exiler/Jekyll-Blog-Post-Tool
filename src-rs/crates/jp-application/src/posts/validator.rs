use jp_domain::authors::Author;
use jp_domain::common::validation::ValidationError;
use jp_domain::posts::FrontMatter;

/// 校验 front matter 字段是否满足 SRS 约束。无状态纯函数。
pub fn validate(front_matter: &FrontMatter, authors: &[Author]) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if front_matter.title.trim().is_empty() {
        errors.push(ValidationError::new("Title", "title 为必填项"));
    }

    if front_matter.date.is_none() {
        errors.push(ValidationError::new("Date", "date 为必填项"));
    }

    if front_matter.categories.len() > 2 {
        errors.push(ValidationError::new("Categories", "categories 最多 2 个"));
    }

    // FR-3.8 (v1.2)：authors 可留空；非空时校验 id 是否存在
    if !front_matter.authors.is_empty() {
        let invalid: Vec<&str> = front_matter
            .authors
            .iter()
            .map(String::as_str)
            .filter(|id| !authors.iter().any(|author| author.id() == *id))
            .collect();

        if !invalid.is_empty() {
            errors.push(ValidationError::new(
                "Authors",
                format!("以下作者不存在：{}", invalid.join(", ")),
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, NaiveDate};
    use jp_domain::posts::{Category, Tag};

    fn date() -> jp_domain::posts::PostDate {
        NaiveDate::from_ymd_opt(2026, 7, 28)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(FixedOffset::east_opt(0).unwrap())
            .single()
            .unwrap()
    }

    fn valid() -> FrontMatter {
        FrontMatter {
            title: "Hello World".to_owned(),
            date: Some(date()),
            authors: vec!["cotes".to_owned()],
            ..FrontMatter::new()
        }
    }

    fn author(id: &str) -> Author {
        Author::new(id, "Cotes", None, None).unwrap()
    }

    #[test]
    fn accepts_complete_front_matter() {
        assert!(validate(&valid(), &[author("cotes")]).is_empty());
    }

    #[test]
    fn requires_title() {
        let mut front_matter = valid();
        front_matter.title = "   ".to_owned();

        let errors = validate(&front_matter, &[author("cotes")]);

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "Title");
        assert_eq!(errors[0].message, "title 为必填项");
    }

    #[test]
    fn requires_date() {
        let mut front_matter = valid();
        front_matter.date = None;

        let errors = validate(&front_matter, &[author("cotes")]);

        assert_eq!(errors[0].field, "Date");
    }

    #[test]
    fn allows_up_to_two_categories() {
        let mut front_matter = valid();
        front_matter.categories = vec![
            Category::new("A").unwrap(),
            Category::new("B").unwrap(),
            Category::new("C").unwrap(),
        ];

        let errors = validate(&front_matter, &[author("cotes")]);

        assert!(errors.iter().any(|e| e.field == "Categories"));
    }

    #[test]
    fn allows_empty_authors() {
        let mut front_matter = valid();
        front_matter.authors.clear();

        assert!(validate(&front_matter, &[author("cotes")]).is_empty());
    }

    #[test]
    fn reports_unknown_author_ids() {
        let mut front_matter = valid();
        front_matter.authors = vec!["ghost".to_owned(), "cotes".to_owned()];

        let errors = validate(&front_matter, &[author("cotes")]);

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "以下作者不存在：ghost");
    }

    #[test]
    fn tags_are_free_form() {
        let mut front_matter = valid();
        front_matter.tags = vec![Tag::new("ANY").unwrap(), Tag::new("任意标签").unwrap()];

        assert!(validate(&front_matter, &[author("cotes")]).is_empty());
    }
}
