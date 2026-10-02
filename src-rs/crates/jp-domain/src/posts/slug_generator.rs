use unicode_normalization::UnicodeNormalization;

use crate::common::error::{DomainError, DomainResult};
use crate::posts::value_objects::Slug;

/// 将博文 title 转换为文件名安全 slug 的领域服务（ADR-006）。
pub fn generate_slug(title: &str) -> DomainResult<Slug> {
    let normalized = title.nfc().collect::<String>();
    if normalized.trim().is_empty() {
        return Err(DomainError::validation_blank("title"));
    }

    let mut builder = String::with_capacity(normalized.len());
    for ch in normalized.trim().chars() {
        // CJK 须先于字母判断，否则中文会落入小写化分支
        if is_cjk(ch) {
            builder.push(ch);
        } else if ch.is_alphabetic() {
            builder.extend(ch.to_lowercase());
        } else if ch.is_numeric() {
            builder.push(ch);
        } else {
            // 空格、`-`、`_` 与其余符号统一压缩为分隔符
            builder.push('-');
        }
    }

    let slug = compress_hyphens(&builder).trim_matches('-').to_owned();
    if slug.is_empty() {
        return Slug::new("untitled");
    }
    Slug::new(&slug)
}

/// CJK Unified Ideographs、Extension A、Compatibility Ideographs。
fn is_cjk(ch: char) -> bool {
    matches!(ch,
        '\u{4E00}'..='\u{9FFF}'
        | '\u{3400}'..='\u{4DBF}'
        | '\u{F900}'..='\u{FAFF}')
}

fn compress_hyphens(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch == '-' && result.ends_with('-') {
            continue;
        }
        result.push(ch);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slug_of(title: &str) -> String {
        generate_slug(title).unwrap().value().to_owned()
    }

    #[test]
    fn english_titles_are_lowercased_and_hyphenated() {
        for (title, expected) in [
            ("Hello World", "hello-world"),
            ("Writing a New Post", "writing-a-new-post"),
            ("  Multiple   Spaces  ", "multiple-spaces"),
            ("Title-With-Hyphens", "title-with-hyphens"),
            ("Title_With_Underscores", "title-with-underscores"),
            ("UPPERCASE", "uppercase"),
            ("Mixed CASE Title", "mixed-case-title"),
        ] {
            assert_eq!(slug_of(title), expected, "title: {title}");
        }
    }

    #[test]
    fn chinese_is_preserved() {
        for (title, expected) in [
            ("中文标题", "中文标题"),
            ("Hello 世界", "hello-世界"),
            ("写作新文章", "写作新文章"),
            ("Jekyll 博客工具", "jekyll-博客工具"),
        ] {
            assert_eq!(slug_of(title), expected, "title: {title}");
        }
    }

    #[test]
    fn symbols_compress_to_single_hyphen() {
        for (title, expected) in [
            ("Title: With Special! @Chars#", "title-with-special-chars"),
            ("A.B.C", "a-b-c"),
            ("Title (With Parens)", "title-with-parens"),
            ("价格/折扣", "价格-折扣"),
            ("A---B---C", "a-b-c"),
            ("Jekyll 3.0 博客工具", "jekyll-3-0-博客工具"),
            ("Post 2026", "post-2026"),
            ("Version 2.0", "version-2-0"),
        ] {
            assert_eq!(slug_of(title), expected, "title: {title}");
        }
    }

    #[test]
    fn leading_and_trailing_hyphens_are_trimmed() {
        for (title, expected) in [
            ("---Leading Hyphens", "leading-hyphens"),
            ("Trailing Hyphens---", "trailing-hyphens"),
            ("###Edge###", "edge"),
        ] {
            assert_eq!(slug_of(title), expected, "title: {title}");
        }
    }

    #[test]
    fn all_symbols_title_falls_back_to_untitled() {
        assert_eq!(slug_of("---"), "untitled");
        assert_eq!(slug_of("!!!"), "untitled");
    }

    #[test]
    fn blank_title_is_rejected() {
        assert!(generate_slug("").is_err());
        assert!(generate_slug("   ").is_err());
    }
}
