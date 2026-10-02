use super::author::Author;
use crate::common::error::DomainResult;

/// 作者仓储契约。同步实现：调用方在 UI 线程之外的工作线程上使用。
pub trait AuthorRepository: Send + Sync {
    fn get_all(&self) -> DomainResult<Vec<Author>>;

    fn save_all(&self, authors: &[Author]) -> DomainResult<()>;
}
