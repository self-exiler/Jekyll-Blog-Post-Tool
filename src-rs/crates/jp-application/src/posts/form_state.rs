use chrono::{NaiveDate, NaiveTime};
use jp_domain::posts::{Category, FrontMatter, PostDate, Slug, Tag};

use crate::error::ApplicationResult;
use crate::posts::time_zone;

/// 表单中已选日期与时间/时区三格的组合：日期可空（FR-3.1 新建时不预填 date）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostFormState {
    pub title: String,
    pub selected_date: Option<NaiveDate>,
    pub selected_time: NaiveTime,
    pub selected_time_zone: String,
    pub category1: String,
    pub category2: String,
    pub tags: String,
    pub description: String,
    pub selected_author_ids: Vec<String>,
}

impl PostFormState {
    /// 新建博文时的空白状态（FR-3.1：date 不预填默认值）。
    pub fn empty(now: PostDate) -> Self {
        Self {
            title: String::new(),
            selected_date: None,
            selected_time: now.time(),
            selected_time_zone: time_zone::format_offset(*now.offset()),
            category1: String::new(),
            category2: String::new(),
            tags: String::new(),
            description: String::new(),
            selected_author_ids: Vec::new(),
        }
    }

    /// Front Matter → 表单。
    pub fn from_front_matter(front_matter: &FrontMatter) -> Self {
        let mut state = Self {
            title: front_matter.title.clone(),
            selected_date: None,
            selected_time: NaiveTime::MIN,
            selected_time_zone: String::new(),
            category1: front_matter
                .categories
                .first()
                .map(|category| category.value().to_owned())
                .unwrap_or_default(),
            category2: front_matter
                .categories
                .get(1)
                .map(|category| category.value().to_owned())
                .unwrap_or_default(),
            // 标签输入用空格分隔（与 to_front_matter 的切分规则对偶）
            tags: front_matter
                .tags
                .iter()
                .map(|tag| tag.value())
                .collect::<Vec<_>>()
                .join(" "),
            description: front_matter.description.clone().unwrap_or_default(),
            selected_author_ids: front_matter.authors.clone(),
        };

        if let Some(date) = &front_matter.date {
            state.selected_date = Some(date.date_naive());
            state.selected_time = date.time();
            state.selected_time_zone = time_zone::format_offset(*date.offset());
        }

        state
    }

    /// 表单 → Front Matter（title/date 缺失等校验由 `validator` 负责）。
    pub fn to_front_matter(&self) -> ApplicationResult<FrontMatter> {
        let mut categories = Vec::new();
        if !self.category1.trim().is_empty() {
            categories.push(Category::new(&self.category1)?);
        }
        if !self.category2.trim().is_empty() {
            categories.push(Category::new(&self.category2)?);
        }

        let mut tags = Vec::new();
        for raw in self.tags.split(' ') {
            let value = raw.trim();
            if !value.is_empty() {
                tags.push(Tag::new(value)?);
            }
        }

        let date = match self.selected_date {
            Some(date) => {
                let naive = date.and_time(self.selected_time);
                let offset = time_zone::parse_or_local(&self.selected_time_zone);
                Some(
                    naive
                        .and_local_timezone(offset)
                        .single()
                        .ok_or_else(|| crate::error::ApplicationError::validation("date 无法解析"))?,
                )
            }
            None => None,
        };

        Ok(FrontMatter {
            title: self.title.clone(),
            date,
            categories,
            tags,
            authors: self.selected_author_ids.clone(),
            description: optional(self.description.as_str()),
            unknown_fields: Default::default(),
        })
    }

    /// 由表单生成文件名预览用的 slug；title 为空时由调用方展示占位提示。
    pub fn build_slug(&self) -> ApplicationResult<Slug> {
        Ok(jp_domain::posts::generate_slug(&self.title)?)
    }
}

/// 空白串归一为 None（对应 .NET `string.IsNullOrWhiteSpace` 判定）。
fn optional(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, Timelike};
    use jp_domain::posts::UnknownFields;

    fn date_at(y: i32, m: u32, d: u32, h: u32, min: u32, offset_hours: i32) -> PostDate {
        let naive = NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap();
        naive
            .and_local_timezone(FixedOffset::east_opt(offset_hours * 3600).unwrap())
            .single()
            .unwrap()
    }

    #[test]
    fn empty_has_no_date_but_keeps_time_and_zone() {
        let state = PostFormState::empty(date_at(2026, 7, 28, 9, 30, 8));

        assert_eq!(state.title, "");
        assert_eq!(state.selected_date, None);
        assert_eq!(state.selected_time, NaiveTime::from_hms_opt(9, 30, 0).unwrap());
        assert_eq!(state.selected_time_zone, "+08:00");
        assert!(state.selected_author_ids.is_empty());
    }

    #[test]
    fn from_front_matter_maps_known_fields() {
        let front_matter = FrontMatter {
            title: "Hello".to_owned(),
            date: Some(date_at(2026, 7, 28, 14, 10, 8)),
            categories: vec![
                Category::new("Blogging").unwrap(),
                Category::new("Tech").unwrap(),
            ],
            tags: vec![Tag::new("Jekyll").unwrap(), Tag::new("WinUI").unwrap()],
            authors: vec!["cotes".to_owned()],
            description: Some("desc".to_owned()),
            unknown_fields: UnknownFields::new(),
        };

        let state = PostFormState::from_front_matter(&front_matter);

        assert_eq!(state.title, "Hello");
        assert_eq!(
            state.selected_date,
            NaiveDate::from_ymd_opt(2026, 7, 28)
        );
        assert_eq!(state.selected_time, NaiveTime::from_hms_opt(14, 10, 0).unwrap());
        assert_eq!(state.selected_time_zone, "+08:00");
        assert_eq!(state.category1, "Blogging");
        assert_eq!(state.category2, "Tech");
        assert_eq!(state.tags, "Jekyll WinUI");
        assert_eq!(state.description, "desc");
        assert_eq!(state.selected_author_ids, vec!["cotes"]);
    }

    #[test]
    fn from_front_matter_without_date_leaves_date_empty() {
        let mut front_matter = FrontMatter::new();
        front_matter.title = "No Date".to_owned();

        let state = PostFormState::from_front_matter(&front_matter);

        assert_eq!(state.selected_date, None);
        assert_eq!(state.selected_time_zone, "");
        assert_eq!(state.description, "");
    }

    #[test]
    fn to_front_matter_splits_tags_and_drops_blank_categories() {
        let state = PostFormState {
            title: "Hello".to_owned(),
            selected_date: NaiveDate::from_ymd_opt(2026, 7, 28),
            selected_time: NaiveTime::from_hms_opt(14, 10, 0).unwrap(),
            selected_time_zone: "+08:00".to_owned(),
            category1: "Blogging".to_owned(),
            category2: "   ".to_owned(),
            tags: "  Jekyll   WinUI  ".to_owned(),
            description: "   ".to_owned(),
            selected_author_ids: vec!["cotes".to_owned()],
        };

        let front_matter = state.to_front_matter().unwrap();

        assert_eq!(front_matter.categories.len(), 1);
        assert_eq!(front_matter.tags.len(), 2);
        assert_eq!(front_matter.tags[1].value(), "WinUI");
        assert_eq!(front_matter.description, None);
        assert_eq!(front_matter.authors, vec!["cotes"]);
        let date = front_matter.date.unwrap();
        assert_eq!(date.hour(), 14);
        assert_eq!(date.offset().local_minus_utc(), 8 * 3600);
    }

    #[test]
    fn to_front_matter_without_date_keeps_date_none() {
        let state = PostFormState::empty(date_at(2026, 7, 28, 9, 0, 0));

        let front_matter = state.to_front_matter().unwrap();

        assert_eq!(front_matter.date, None);
    }

    #[test]
    fn round_trip_preserves_known_fields() {
        let original = FrontMatter {
            title: "Round Trip".to_owned(),
            date: Some(date_at(2026, 7, 28, 14, 10, 8)),
            categories: vec![Category::new("A").unwrap(), Category::new("B").unwrap()],
            tags: vec![Tag::new("tag1").unwrap(), Tag::new("tag2").unwrap()],
            authors: vec!["cotes".to_owned(), "admin".to_owned()],
            description: Some("Round trip test".to_owned()),
            unknown_fields: UnknownFields::new(),
        };

        let parsed = PostFormState::from_front_matter(&original)
            .to_front_matter()
            .unwrap();

        assert_eq!(parsed.title, original.title);
        assert_eq!(parsed.date, original.date);
        assert_eq!(parsed.categories, original.categories);
        assert_eq!(parsed.tags, original.tags);
        assert_eq!(parsed.authors, original.authors);
        assert_eq!(parsed.description, original.description);
    }

    #[test]
    fn to_front_matter_rejects_blank_tag_like_value() {
        let state = PostFormState {
            category1: "\t".to_owned(),
            ..PostFormState::empty(date_at(2026, 1, 1, 0, 0, 0))
        };

        // 空白分类被跳过而非报错
        assert!(state.to_front_matter().unwrap().categories.is_empty());
    }
}
