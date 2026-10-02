use std::fmt;

/// 领域/基础设施错误的统一载体。`Display` 输出即展示给用户的中文文本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// 输入校验失败，字符串为展示消息。
    Validation(String),
    /// 文件系统访问失败。
    Io { path: String, message: String },
}

impl DomainError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn validation_blank(field: &str) -> Self {
        Self::Validation(format!("{field} 不能为空"))
    }

    pub fn io(path: impl Into<String>, error: &std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            message: error.to_string(),
        }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => f.write_str(message),
            Self::Io { path, message } => write!(f, "{message}（路径：{path}）"),
        }
    }
}

impl std::error::Error for DomainError {}

impl From<std::io::Error> for DomainError {
    fn from(value: std::io::Error) -> Self {
        Self::Io {
            path: String::new(),
            message: value.to_string(),
        }
    }
}

pub type DomainResult<T> = Result<T, DomainError>;
