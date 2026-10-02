//! 当前项目上下文：可跨线程读写的共享状态（对应原 `ProjectContext : ObservableObject`）。
//!
//! 原版靠 `PropertyChanged` 通知，这里改为「页面每次进入时主动同步」，
//! 与 .NET 侧 `RefreshFromContext()` 的用法一致，省去订阅与解订阅。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use jp_domain::projects::BlogProject;
use jp_infrastructure::filesystem::yaml_author_repository::PathResolver;

#[derive(Clone, Default)]
pub struct ProjectContext {
    current: Arc<Mutex<Option<BlogProject>>>,
}

impl ProjectContext {
    pub fn current(&self) -> Option<BlogProject> {
        self.lock().clone()
    }

    pub fn path(&self) -> Option<PathBuf> {
        self.current().map(|project| project.path().to_path_buf())
    }

    pub fn set(&self, project: Option<BlogProject>) {
        *self.lock() = project;
    }

    /// 作者文件路径惰性解析器：支持运行期切换项目（ADR-005）。
    pub fn authors_path_resolver(&self) -> PathResolver {
        let current = Arc::clone(&self.current);
        Arc::new(move || {
            current
                .lock()
                .ok()
                .and_then(|project| project.as_ref().map(BlogProject::authors_file_path))
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<BlogProject>> {
        self.current
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
