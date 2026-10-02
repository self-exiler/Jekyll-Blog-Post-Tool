//! 领域层：博文实体、front matter 值对象、slug 规则与仓储契约。
//! 不依赖任何 UI 或 IO 实现。

pub mod authors;
pub mod common;
pub mod posts;
pub mod projects;

pub use common::error::DomainError;
pub use common::validation::ValidationError;
