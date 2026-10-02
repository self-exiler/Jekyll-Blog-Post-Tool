use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};

use crate::error::{ApplicationError, ApplicationResult};

/// 通用 JSON 文件读写，消除 AI 配置与默认项目设置的重复 I/O 逻辑。
///
/// 字段命名沿用 .NET 侧的 camelCase，两个实现可共用同一份 `%APPDATA%\JekyllPostTool`。
#[derive(Debug, Clone)]
pub struct JsonFileStore {
    file_path: PathBuf,
}

impl JsonFileStore {
    pub fn new(app_data_directory: &Path, file_name: &str) -> Self {
        Self {
            file_path: app_data_directory.join(file_name),
        }
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// 文件不存在返回 `None`；内容无法解析时报错而非静默返回默认值。
    pub fn read<T: DeserializeOwned>(&self) -> ApplicationResult<Option<T>> {
        if !self.file_path.exists() {
            return Ok(None);
        }

        let text = std::fs::read_to_string(&self.file_path).map_err(|error| {
            ApplicationError::Domain(jp_domain::common::error::DomainError::io(
                self.file_path.display().to_string(),
                &error,
            ))
        })?;

        serde_json::from_str(&text).map(Some).map_err(|error| {
            ApplicationError::validation(format!("配置文件解析失败：{error}"))
        })
    }

    pub fn write<T: Serialize>(&self, data: &T) -> ApplicationResult<()> {
        if let Some(directory) = self.file_path.parent() {
            if !directory.as_os_str().is_empty() {
                std::fs::create_dir_all(directory).map_err(|error| {
                    ApplicationError::Domain(jp_domain::common::error::DomainError::io(
                        directory.display().to_string(),
                        &error,
                    ))
                })?;
            }
        }

        let text = serde_json::to_string_pretty(data)
            .map_err(|error| ApplicationError::validation(format!("配置文件写入失败：{error}")))?;

        std::fs::write(&self.file_path, text).map_err(|error| {
            ApplicationError::Domain(jp_domain::common::error::DomainError::io(
                self.file_path.display().to_string(),
                &error,
            ))
        })
    }
}
