use jp_domain::common::error::DomainError;

/// 应用层错误。`Display` 输出即展示给用户的中文文本，UI 直接呈现无需再加工。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationError {
    Domain(DomainError),
    AuthorAlreadyExists(String),
    AuthorNotFound(String),
    /// 冲突存在但调用方未给出处理方式。
    ConflictResolutionRequired,
    ConflictResolutionUnsupported,
    /// 未选择博客项目。
    NoProject,
    Validation(String),
}

impl ApplicationError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }
}

impl std::fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Domain(error) => write!(f, "{error}"),
            Self::AuthorAlreadyExists(id) => write!(f, "作者 id '{id}' 已存在"),
            Self::AuthorNotFound(id) => write!(f, "作者 id '{id}' 不存在"),
            Self::ConflictResolutionRequired => f.write_str("需要选择冲突处理方式"),
            Self::ConflictResolutionUnsupported => f.write_str("不支持的冲突处理方式"),
            Self::NoProject => f.write_str("未选择博客项目，无法保存作者信息。"),
            Self::Validation(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ApplicationError {}

impl From<DomainError> for ApplicationError {
    fn from(value: DomainError) -> Self {
        Self::Domain(value)
    }
}

pub type ApplicationResult<T> = Result<T, ApplicationError>;
