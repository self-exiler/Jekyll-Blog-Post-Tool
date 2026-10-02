use std::path::{Path, PathBuf};

use super::front_matter::{FrontMatter, PostDate};
use super::value_objects::Slug;
use crate::common::error::{DomainError, DomainResult};
use crate::common::paths;

/// 博文实体：文件路径 + front matter + 正文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Post {
    file_path: PathBuf,
    front_matter: FrontMatter,
    body: String,
}

impl Post {
    pub fn new(
        file_path: impl AsRef<Path>,
        front_matter: FrontMatter,
        body: impl Into<String>,
    ) -> DomainResult<Self> {
        let file_path = file_path.as_ref();
        if paths::file_name(file_path).is_empty() {
            return Err(DomainError::validation_blank("filePath"));
        }

        Ok(Self {
            file_path: file_path.to_path_buf(),
            front_matter,
            body: body.into(),
        })
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    pub fn front_matter(&self) -> &FrontMatter {
        &self.front_matter
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn file_name(&self) -> String {
        paths::file_name(&self.file_path)
    }

    /// 由日期与 slug 构造博文文件名（`YYYY-MM-DD-slug.md`）。
    /// 日期取 front matter date 自身时区下的日历日，数字恒为 ASCII。
    pub fn build_file_name(date: &PostDate, slug: &Slug) -> String {
        format!("{}-{}.md", date.format("%Y-%m-%d"), slug.value())
    }

    /// `build_file_name` 的逆变换。无合法 `YYYY-MM-DD-` 前缀时返回去扩展名的原文件名，
    /// 前缀须满 4-2-2 数字形态且后缀非空，避免误剥标题恰似该形态的文件名。
    pub fn try_extract_slug(file_name: &str) -> String {
        let name = paths::file_name_without_extension_str(file_name);
        let bytes = name.as_bytes();

        let has_date_prefix = bytes.len() >= 12
            && bytes[..4].iter().all(u8::is_ascii_digit)
            && bytes[4] == b'-'
            && bytes[5..7].iter().all(u8::is_ascii_digit)
            && bytes[7] == b'-'
            && bytes[8..10].iter().all(u8::is_ascii_digit)
            && bytes[10] == b'-';

        if has_date_prefix {
            name[11..].to_owned()
        } else {
            name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, NaiveDate};

    fn date(y: i32, mo: u32, d: u32) -> PostDate {
        let naive = NaiveDate::from_ymd_opt(y, mo, d)
            .and_then(|day| day.and_hms_opt(14, 10, 0))
            .unwrap();
        naive
            .and_local_timezone(FixedOffset::east_opt(8 * 3600).unwrap())
            .single()
            .unwrap()
    }

    #[test]
    fn build_file_name_formats_date_and_slug() {
        let slug = Slug::new("writing-a-new-post").unwrap();

        assert_eq!(
            Post::build_file_name(&date(2026, 7, 28), &slug),
            "2026-07-28-writing-a-new-post.md"
        );
    }

    #[test]
    fn build_file_name_preserves_chinese_slug() {
        let slug = Slug::new("用-winui3-制作博客工具").unwrap();

        assert_eq!(
            Post::build_file_name(&date(2026, 7, 27), &slug),
            "2026-07-27-用-winui3-制作博客工具.md"
        );
    }

    #[test]
    fn try_extract_slug_strips_date_prefix() {
        assert_eq!(
            Post::try_extract_slug("2026-07-28-writing-a-new-post.md"),
            "writing-a-new-post"
        );
        assert_eq!(
            Post::try_extract_slug("2026-07-27-用-winui3-制作博客工具.md"),
            "用-winui3-制作博客工具"
        );
        assert_eq!(Post::try_extract_slug("2026-01-01-a.md"), "a");
    }

    #[test]
    fn try_extract_slug_keeps_name_without_valid_prefix() {
        assert_eq!(Post::try_extract_slug("text-and-typography.md"), "text-and-typography");
        assert_eq!(Post::try_extract_slug("2026-7-28-short.md"), "2026-7-28-short");
        assert_eq!(Post::try_extract_slug("2026-07-28-.md"), "2026-07-28-");
        assert_eq!(Post::try_extract_slug("/some/dir/2026-07-28-nested.md"), "nested");
    }

    #[test]
    fn new_rejects_blank_path() {
        let error = Post::new("", FrontMatter::new(), "").unwrap_err();

        assert!(matches!(error, DomainError::Validation(_)));
    }

    #[test]
    fn file_name_extracts_last_segment() {
        let post = Post::new(
            Path::new("C:/blog/_posts/2026-07-28-hello.md"),
            FrontMatter::new(),
            "",
        )
        .unwrap();

        assert_eq!(post.file_name(), "2026-07-28-hello.md");
    }
}
