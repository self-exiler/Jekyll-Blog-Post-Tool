use std::path::{Path, PathBuf};

use jp_domain::common::validation::ValidationError;

use super::conflict_resolver::ConflictResult;

/// 博文保存操作结果。状态与载荷由变体本身约束，不会出现「Saved 却带 Errors」这类组合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostOperationResult {
    Saved {
        file_path: PathBuf,
        warnings: Vec<String>,
    },
    ValidationFailed {
        errors: Vec<ValidationError>,
    },
    Conflict {
        conflict: ConflictResult,
    },
    ModifiedExternally,
}

impl PostOperationResult {
    pub fn success(file_path: PathBuf) -> Self {
        Self::Saved { file_path, warnings: Vec::new() }
    }

    pub fn success_with_warnings(file_path: PathBuf, warnings: Vec<String>) -> Self {
        Self::Saved { file_path, warnings }
    }

    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self::ValidationFailed { errors }
    }

    pub fn with_conflict(conflict: ConflictResult) -> Self {
        Self::Conflict { conflict }
    }

    pub fn modified_externally() -> Self {
        Self::ModifiedExternally
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Saved { .. })
    }

    pub fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict { .. })
    }

    pub fn is_modified_externally(&self) -> bool {
        matches!(self, Self::ModifiedExternally)
    }

    pub fn file_path(&self) -> Option<&Path> {
        match self {
            Self::Saved { file_path, .. } => Some(file_path),
            _ => None,
        }
    }

    pub fn errors(&self) -> &[ValidationError] {
        match self {
            Self::ValidationFailed { errors } => errors,
            _ => &[],
        }
    }

    pub fn warnings(&self) -> &[String] {
        match self {
            Self::Saved { warnings, .. } => warnings,
            _ => &[],
        }
    }

    pub fn conflict(&self) -> Option<&ConflictResult> {
        match self {
            Self::Conflict { conflict } => Some(conflict),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_predicates_align_with_variants() {
        let saved = PostOperationResult::success(PathBuf::from("/tmp/a.md"));
        assert!(saved.is_success());
        assert_eq!(saved.file_path(), Some(Path::new("/tmp/a.md")));
        assert!(saved.errors().is_empty());

        let conflict = PostOperationResult::with_conflict(ConflictResult::new(
            PathBuf::from("/tmp/b.md"),
            Some(1),
        ));
        assert!(conflict.is_conflict());
        assert_eq!(conflict.file_path(), None);

        assert!(PostOperationResult::modified_externally().is_modified_externally());
        assert!(PostOperationResult::failure(vec![ValidationError::new("Title", "title 为必填项")])
            .errors()
            .len()
            == 1);
    }
}
