use crate::common::error::{DomainError, DomainResult};

/// 作者实体，按项目隔离，存于 `_data/authors.yml`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author {
    id: String,
    name: String,
    twitter: Option<String>,
    url: Option<String>,
}

impl Author {
    pub fn new(
        id: &str,
        name: &str,
        twitter: Option<&str>,
        url: Option<&str>,
    ) -> DomainResult<Self> {
        Ok(Self {
            id: require_trimmed("id", id)?,
            name: require_trimmed("name", name)?,
            twitter: optional_trimmed(twitter),
            url: optional_trimmed(url),
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn twitter(&self) -> Option<&str> {
        self.twitter.as_deref()
    }

    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}

fn require_trimmed(field: &str, value: &str) -> DomainResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DomainError::validation_blank(field));
    }
    Ok(trimmed.to_owned())
}

/// 空白串归一为 None（与 .NET `string.IsNullOrWhiteSpace` 判定一致）。
fn optional_trimmed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|trimmed| !trimmed.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trims_and_keeps_values() {
        let author = Author::new(" cotes ", " Cotes ", Some("  cotes201 "), Some("  ")).unwrap();

        assert_eq!(author.id(), "cotes");
        assert_eq!(author.name(), "Cotes");
        assert_eq!(author.twitter(), Some("cotes201"));
        assert_eq!(author.url(), None);
    }

    #[test]
    fn new_rejects_blank_id_or_name() {
        assert!(Author::new("", "Cotes", None, None).is_err());
        assert!(Author::new("   ", "Cotes", None, None).is_err());
        assert!(Author::new("cotes", "   ", None, None).is_err());
    }
}
