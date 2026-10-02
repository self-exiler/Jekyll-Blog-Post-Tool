use std::path::{Path, PathBuf};
use std::sync::Arc;

use jp_domain::common::paths;
use jp_domain::posts::PostRepository;
use jp_domain::projects::BlogProject;

use crate::error::{ApplicationError, ApplicationResult};

/// 文件名冲突时的用户选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolutionKind {
    AutoSuffix,
    Overwrite,
}

/// 检测博文文件名冲突并提供候选方案。
pub struct FilenameConflictResolver {
    post_repository: Arc<dyn PostRepository>,
}

impl FilenameConflictResolver {
    pub fn new(post_repository: Arc<dyn PostRepository>) -> Self {
        Self { post_repository }
    }

    pub fn check(&self, project: &BlogProject, file_name: &str) -> ConflictResult {
        let file_path = project.posts_directory().join(file_name);
        if self.post_repository.exists(&file_path) {
            let suffix = self.find_next_available_suffix(project, file_name);
            return ConflictResult::new(file_path, Some(suffix));
        }
        ConflictResult::new(file_path, None)
    }

    fn find_next_available_suffix(&self, project: &BlogProject, file_name: &str) -> i64 {
        let file_path = project.posts_directory().join(file_name);
        let mut suffix = 1;
        while self.post_repository.exists(&paths::append_suffix(&file_path, suffix)) {
            suffix += 1;
        }
        suffix
    }
}

/// 冲突检测结果：`auto_suffix` 为冲突时首个可用序号，无冲突时为 `None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictResult {
    file_path: PathBuf,
    auto_suffix: Option<i64>,
}

impl ConflictResult {
    pub fn new(file_path: PathBuf, auto_suffix: Option<i64>) -> Self {
        Self { file_path, auto_suffix }
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    pub fn auto_suffix(&self) -> Option<i64> {
        self.auto_suffix
    }

    pub fn has_conflict(&self) -> bool {
        self.auto_suffix.is_some()
    }

    pub fn file_name(&self) -> String {
        paths::file_name(&self.file_path)
    }

    pub fn resolve(&self, resolution: Option<ConflictResolutionKind>) -> ApplicationResult<PathBuf> {
        if !self.has_conflict() {
            return Ok(self.file_path.clone());
        }

        match resolution {
            Some(ConflictResolutionKind::AutoSuffix) => {
                Ok(paths::append_suffix(&self.file_path, self.auto_suffix.unwrap_or(1)))
            }
            Some(ConflictResolutionKind::Overwrite) => Ok(self.file_path.clone()),
            None => Err(ApplicationError::ConflictResolutionRequired),
        }
    }
}
