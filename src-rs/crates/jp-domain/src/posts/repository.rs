use std::path::{Path, PathBuf};

use super::front_matter::FrontMatter;
use super::post::Post;
use crate::common::error::DomainResult;

/// 博文文件的读盘快照：全文与解析结果来自同一次 IO。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostRead {
    pub file_path: PathBuf,
    pub content: String,
    pub front_matter: FrontMatter,
    pub body: String,
}

impl PostRead {
    pub fn to_post(&self) -> Post {
        Post::new(&self.file_path, self.front_matter.clone(), self.body.clone())
            .expect("PostRead 来自同一次读盘，file_path 必然有效")
    }
}

/// 博文仓储契约。同步实现：调用方在 UI 线程之外的工作线程上使用。
pub trait PostRepository: Send + Sync {
    fn exists(&self, file_path: &Path) -> bool;

    /// 文件不存在时静默返回，其余 IO 错误上抛。
    fn delete(&self, file_path: &Path) -> DomainResult<()>;

    /// 单次读盘：原文与解析结果同源，保证外部修改检测的基线哈希与展示内容一致；
    /// 文件不存在返回 `None`。
    fn read(&self, file_path: &Path) -> DomainResult<Option<PostRead>>;

    fn save(&self, post: &Post) -> DomainResult<()>;

    /// 读取原始全文（不解析）；文件不存在返回 `None`。
    fn read_all_text(&self, file_path: &Path) -> DomainResult<Option<String>>;
}
