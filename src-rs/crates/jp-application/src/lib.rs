//! 应用层：用例编排、表单映射、字段校验与配置持久化。

pub mod ai;
pub mod authors;
pub mod error;
pub mod json_store;
pub mod posts;
pub mod projects;

#[cfg(test)]
mod test_support;

pub use error::{ApplicationError, ApplicationResult};
