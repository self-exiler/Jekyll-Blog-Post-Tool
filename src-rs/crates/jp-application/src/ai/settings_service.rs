use std::path::Path;

use super::settings::AiSettings;
use crate::error::ApplicationResult;
use crate::json_store::JsonFileStore;

/// 管理 AI 配置的读写，直接持久化到 `ai.json`（FR-7.1）。
#[derive(Debug)]
pub struct AiSettingsService {
    store: JsonFileStore,
}

impl AiSettingsService {
    pub fn new(app_data_directory: &Path) -> Self {
        Self {
            store: JsonFileStore::new(app_data_directory, "ai.json"),
        }
    }

    pub fn get(&self) -> ApplicationResult<AiSettings> {
        Ok(self.store.read()?.unwrap_or_default())
    }

    pub fn set(&self, settings: &AiSettings) -> ApplicationResult<()> {
        self.store.write(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{cleanup, temp_dir};

    fn complete() -> AiSettings {
        AiSettings {
            base_url: "https://api.openai.com/v1".to_owned(),
            api_key: "sk-test".to_owned(),
            model: "gpt-4o-mini".to_owned(),
        }
    }

    #[test]
    fn returns_defaults_when_file_absent() {
        let directory = temp_dir("ai-absent");
        let service = AiSettingsService::new(&directory);

        let settings = service.get().unwrap();

        assert!(settings == AiSettings::default());
        assert!(!settings.is_configured());
        cleanup(&directory);
    }

    #[test]
    fn round_trips_through_disk() {
        let directory = temp_dir("ai-roundtrip");
        let service = AiSettingsService::new(&directory);

        service.set(&complete()).unwrap();

        assert_eq!(service.get().unwrap(), complete());
        assert!(directory.join("ai.json").exists());
        cleanup(&directory);
    }

    #[test]
    fn creates_missing_directory_on_write() {
        let directory = temp_dir("ai-mkdir").join("nested");
        let service = AiSettingsService::new(&directory);

        service.set(&complete()).unwrap();

        assert!(directory.join("ai.json").exists());
        cleanup(&directory.parent().unwrap());
    }

    #[test]
    fn empty_settings_clear_stored_values() {
        let directory = temp_dir("ai-clear");
        let service = AiSettingsService::new(&directory);
        service.set(&complete()).unwrap();

        service.set(&AiSettings::default()).unwrap();

        assert!(!service.get().unwrap().is_configured());
        cleanup(&directory);
    }
}
