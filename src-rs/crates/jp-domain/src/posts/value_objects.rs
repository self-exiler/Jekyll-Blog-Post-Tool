use crate::common::error::{DomainError, DomainResult};

/// 单值文本值对象：非空、去首尾空白，原样保留大小写与中英文。
macro_rules! text_value_object {
    ($(#[$meta:meta])* $name:ident, $field:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: &str) -> DomainResult<Self> {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(DomainError::validation_blank($field));
                }
                Ok(Self(trimmed.to_owned()))
            }

            pub fn value(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

text_value_object! {
    /// 从 title 生成的文件名片段。
    Slug, "Slug"
}

text_value_object! {
    /// 分类值对象，原样保留用户输入。
    Category, "Category"
}

text_value_object! {
    /// 标签值对象，原样保留大小写与中英文。
    Tag, "Tag"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trims_value() {
        assert_eq!(Category::new("  Blogging  ").unwrap().value(), "Blogging");
        assert_eq!(Tag::new(" WinUI ").unwrap().value(), "WinUI");
    }

    #[test]
    fn blank_value_is_rejected() {
        assert!(Slug::new("").is_err());
        assert!(Slug::new("   ").is_err());
        assert!(Category::new("\t").is_err());
        assert!(Tag::new("\n").is_err());
    }

    #[test]
    fn case_is_preserved() {
        assert_eq!(Tag::new("WinUI").unwrap().value(), "WinUI");
        assert_eq!(Tag::new("winui").unwrap().value(), "winui");
    }
}
