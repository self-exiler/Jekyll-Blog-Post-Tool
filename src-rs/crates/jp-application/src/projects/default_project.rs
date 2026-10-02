use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ApplicationResult;
use crate::json_store::JsonFileStore;

/// 管理默认项目路径的读写，直接持久化到 `settings.json`（ADR-005：全局仅记一个项目）。
#[derive(Debug)]
pub struct DefaultProjectSettingService {
    store: JsonFileStore,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct DefaultProjectSettings {
    default_project_path: Option<String>,
}

impl DefaultProjectSettingService {
    pub fn new(app_data_directory: &Path) -> Self {
        Self {
            store: JsonFileStore::new(app_data_directory, "settings.json"),
        }
    }

    pub fn get(&self) -> ApplicationResult<Option<String>> {
        Ok(self.store.read()?.and_then(|settings: DefaultProjectSettings| settings.default_project_path))
    }

    pub fn set(&self, path: Option<&str>) -> ApplicationResult<()> {
        self.store.write(&DefaultProjectSettings {
            default_project_path: path.map(str::to_owned),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{cleanup, temp_dir};

    #[test]
    fn returns_none_when_unconfigured() {
        let directory = temp_dir("project-absent");

        assert_eq!(DefaultProjectSettingService::new(&directory).get().unwrap(), None);
        cleanup(&directory);
    }

    #[test]
    fn round_trips_project_path() {
        let directory = temp_dir("project-roundtrip");
        let service = DefaultProjectSettingService::new(&directory);

        service.set(Some("C:/blog")).unwrap();

        assert_eq!(service.get().unwrap().as_deref(), Some("C:/blog"));
        cleanup(&directory);
    }

    #[test]
    fn set_none_clears_stored_path() {
        let directory = temp_dir("project-clear");
        let service = DefaultProjectSettingService::new(&directory);
        service.set(Some("C:/blog")).unwrap();

        service.set(None).unwrap();

        assert_eq!(service.get().unwrap(), None);
        cleanup(&directory);
    }

    #[test]
    fn reads_json_written_by_dotnet() {
        let directory = temp_dir("project-dotnet");
        std::fs::write(
            directory.join("settings.json"),
            r#"{"defaultProjectPath":"C:\\blog"}"#,
        )
        .unwrap();

        assert_eq!(
            DefaultProjectSettingService::new(&directory).get().unwrap().as_deref(),
            Some("C:\\blog")
        );
        cleanup(&directory);
    }
}
