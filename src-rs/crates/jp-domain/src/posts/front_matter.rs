use chrono::{DateTime, FixedOffset};

use super::unknown_fields::UnknownFields;
use super::value_objects::{Category, Tag};

/// front matter 中的日期时间（含时区偏移），Chirpy 形态为 `YYYY-MM-DD HH:MM:SS ±hh:mm`。
pub type PostDate = DateTime<FixedOffset>;

/// 博文 front matter 值对象：v1 最小集强类型字段 + 未知字段保序集合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontMatter {
    pub title: String,
    pub date: Option<PostDate>,
    pub categories: Vec<Category>,
    pub tags: Vec<Tag>,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub unknown_fields: UnknownFields,
}

impl Default for FrontMatter {
    fn default() -> Self {
        Self {
            title: String::new(),
            date: None,
            categories: Vec::new(),
            tags: Vec::new(),
            authors: Vec::new(),
            description: None,
            unknown_fields: UnknownFields::new(),
        }
    }
}

impl FrontMatter {
    pub fn new() -> Self {
        Self::default()
    }
}
