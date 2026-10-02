use serde::{Deserialize, Serialize};

/// OpenAI 兼容 API 的配置（FR-7.1）。字段名与 .NET 侧 camelCase JSON 保持一致。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AiSettings {
    /// API 基址，如 `https://api.openai.com/v1`。
    pub base_url: String,
    /// API Key（Bearer 令牌）。
    pub api_key: String,
    /// 模型名，如 `gpt-4o-mini`。
    pub model: String,
}

impl AiSettings {
    /// 是否已填写完整可调用。
    pub fn is_configured(&self) -> bool {
        !(self.base_url.trim().is_empty() || self.api_key.trim().is_empty() || self.model.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_all_three_values() {
        assert!(!AiSettings::default().is_configured());
        assert!(!AiSettings { base_url: "https://api.openai.com/v1".to_owned(), ..Default::default() }.is_configured());
        assert!(AiSettings {
            base_url: "https://api.openai.com/v1".to_owned(),
            api_key: "sk-test".to_owned(),
            model: "gpt-4o-mini".to_owned(),
        }
        .is_configured());
    }

    #[test]
    fn whitespace_only_value_counts_as_missing() {
        let settings = AiSettings {
            base_url: "https://api.openai.com/v1".to_owned(),
            api_key: "   ".to_owned(),
            model: "gpt-4o-mini".to_owned(),
        };

        assert!(!settings.is_configured());
    }

    #[test]
    fn serializes_with_dotnet_camel_case_names() {
        let json = serde_json::to_string(&AiSettings {
            base_url: "u".to_owned(),
            api_key: "k".to_owned(),
            model: "m".to_owned(),
        })
        .unwrap();

        assert!(json.contains("\"baseUrl\":\"u\""), "{json}");
        assert!(json.contains("\"apiKey\":\"k\""), "{json}");
        assert!(json.contains("\"model\":\"m\""), "{json}");
    }

    #[test]
    fn reads_json_written_by_dotnet() {
        let settings: AiSettings =
            serde_json::from_str(r#"{"baseUrl":"u","apiKey":"k","model":"m"}"#).unwrap();

        assert_eq!(settings.base_url, "u");
        assert_eq!(settings.api_key, "k");
    }

    #[test]
    fn missing_fields_default_to_empty() {
        let settings: AiSettings = serde_json::from_str(r#"{"baseUrl":"u"}"#).unwrap();

        assert_eq!(settings.base_url, "u");
        assert_eq!(settings.model, "");
    }
}
