//! 基于 OpenAI 兼容 Chat Completions API 的关键字提取实现（FR-7.2~7.3）。
//! 每次调用读取最新配置，用户在高级功能页改动后立即生效。

use std::sync::Arc;
use std::time::Duration;

use jp_application::ai::AiSettingsService;
use jp_application::error::{ApplicationError, ApplicationResult};
use serde::Deserialize;
use serde_json::json;

/// 关键字提取的调用契约：同步实现，调用方（UI 侧）放到工作线程上执行。
pub trait KeywordExtractor: Send + Sync {
    fn extract_keywords(&self, body: &str, max_count: usize) -> ApplicationResult<Vec<String>>;
}

pub struct OpenAiClient {
    settings_service: AiSettingsService,
    agent: ureq::Agent,
}

impl OpenAiClient {
    pub fn new(settings_service: AiSettingsService) -> Self {
        Self::with_agent(
            settings_service,
            ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(15))
                .timeout_read(Duration::from_secs(120))
                .build(),
        )
    }

    /// 注入自定义 agent（测试用 fake transport / 调整超时）。
    pub fn with_agent(settings_service: AiSettingsService, agent: ureq::Agent) -> Self {
        Self { settings_service, agent }
    }

    pub fn into_shared(self) -> Arc<dyn KeywordExtractor> {
        Arc::new(self)
    }
}

impl KeywordExtractor for OpenAiClient {
    fn extract_keywords(&self, body: &str, max_count: usize) -> ApplicationResult<Vec<String>> {
        if body.trim().is_empty() {
            return Err(ApplicationError::validation("body 不能为空"));
        }

        let settings = self.settings_service.get()?;
        if !settings.is_configured() {
            return Err(ApplicationError::validation(
                "尚未配置 AI API（Base URL / API Key / 模型）。",
            ));
        }

        let endpoint = build_endpoint(&settings.base_url);
        let system_prompt = format!(
            "你是一个关键字提取助手。从给定的 markdown 博文正文中提取最多 {max_count} 个最能代表内容的关键字。\
关键字应为简短的词或短语（中文或英文，跟随正文语言）。仅返回以英文逗号分隔的关键字列表，不要编号、不要解释、不要其他内容。"
        );

        let payload = json!({
            "model": settings.model,
            "temperature": 0.2,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": body }
            ]
        });

        let response = self
            .agent
            .post(&endpoint)
            .set("Authorization", &format!("Bearer {}", settings.api_key.trim()))
            .send_json(&payload)
            .map_err(|error| ApplicationError::validation(format!("AI API 调用失败：{error}")))?;

        let parsed: ChatResponse = response
            .into_json()
            .map_err(|error| ApplicationError::validation(format!("AI 响应解析失败：{error}")))?;

        let content = parsed
            .choices
            .as_deref()
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.message.as_ref())
            .and_then(|message| message.content.clone())
            .unwrap_or_default();

        Ok(parse_keywords(&content, max_count))
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Option<Vec<Choice>>,
}

#[derive(Deserialize)]
struct Choice {
    message: Option<ChatMessage>,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

/// 由 base URL 推出 chat completions 端点。
fn build_endpoint(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.to_ascii_lowercase().ends_with("/chat/completions") {
        return trimmed.to_owned();
    }

    // 路径段级判断是否已含版本段，避免 "/v1-proxy" 之类子串误判；否则补 /v1
    let has_version_segment = trimmed
        .split('/')
        .filter(|segment| !segment.is_empty())
        .any(|segment| segment.eq_ignore_ascii_case("v1"));

    if has_version_segment {
        format!("{trimmed}/chat/completions")
    } else {
        format!("{trimmed}/v1/chat/completions")
    }
}

/// 兼容中英文逗号、换行、顿号（与 .NET 的切分集合一致）；
/// 大小写不敏感去重，保留首次出现的大小写。
fn parse_keywords(content: &str, max_count: usize) -> Vec<String> {
    let mut keywords: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    for token in content
        .split([',', '，', '\n', '\r', '、'])
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let cleaned = clean_token(token);
        if cleaned.is_empty() {
            continue;
        }

        if seen.iter().any(|existing| existing.eq_ignore_ascii_case(&cleaned)) {
            continue;
        }

        seen.push(cleaned.clone());
        keywords.push(cleaned);
        if keywords.len() >= max_count {
            break;
        }
    }

    keywords
}

/// 去除前导序号（"1." / "1:" / "1)"）与包裹的引号。
fn clean_token(token: &str) -> String {
    let token = token.trim();
    let digits = token.chars().take_while(|ch| ch.is_ascii_digit()).count();

    // 数字个数为 ASCII 字节数，按字节切片安全
    let rest = if digits > 0 {
        match token[digits..].chars().next() {
            Some(marker) if matches!(marker, '.' | ':' | ')') => &token[digits + marker.len_utf8()..],
            _ => token,
        }
    } else {
        token
    };

    let trimmed = rest.trim_matches([' ', '\t', '"', '\'', '“', '”', '‘', '’']);
    // 单个数字是「2、Rust」被顿号切开后残留的序号（.NET 会把它当关键词，这里丢弃）；
    // 多位数字（如 "2026"）是人读关键词，原样保留。
    if trimmed.len() == 1 && trimmed.is_ascii() && trimmed.as_bytes()[0].is_ascii_digit() {
        return String::new();
    }
    trimmed.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_appends_version_and_path() {
        assert_eq!(
            build_endpoint("https://api.openai.com"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            build_endpoint("https://api.openai.com/"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            build_endpoint("https://api.openai.com/v1"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            build_endpoint("https://gw.internal/v1/chat/completions"),
            "https://gw.internal/v1/chat/completions"
        );
        assert_eq!(
            build_endpoint("https://gw.internal/V1"),
            "https://gw.internal/V1/chat/completions"
        );
        // "v1" 必须是独立路径段，子串命中不算
        assert_eq!(
            build_endpoint("https://gw.internal/v1-proxy"),
            "https://gw.internal/v1-proxy/v1/chat/completions"
        );
    }

    #[test]
    fn keywords_split_on_mixed_separators() {
        assert_eq!(
            parse_keywords("Jekyll， Rust、博客\n写作", 5),
            vec!["Jekyll", "Rust", "博客", "写作"]
        );
    }

    #[test]
    fn numbered_list_prefix_is_stripped() {
        assert_eq!(
            parse_keywords("1. Jekyll\n2、Rust\n3) 博客\n4: 写作", 5),
            vec!["Jekyll", "Rust", "博客", "写作"]
        );
    }

    #[test]
    fn quotes_are_trimmed() {
        assert_eq!(parse_keywords("“Jekyll”, 'Rust', ”博客“", 5), vec!["Jekyll", "Rust", "博客"]);
    }

    #[test]
    fn dedupe_is_case_insensitive_and_keeps_first_spelling() {
        assert_eq!(parse_keywords("Rust, rust, RUST, Go", 5), vec!["Rust", "Go"]);
    }

    #[test]
    fn respects_max_count() {
        assert_eq!(parse_keywords("a, b, c, d", 2), vec!["a", "b"]);
    }

    #[test]
    fn empty_or_punctuation_only_content_yields_nothing() {
        assert!(parse_keywords("", 5).is_empty());
        assert!(parse_keywords("  , , 、 ", 5).is_empty());
    }

    #[test]
    fn chinese_digit_token_keeps_digits() {
        // "2026" 之类纯数字没有序号分隔符，应原样保留
        assert_eq!(parse_keywords("2026, Rust", 5), vec!["2026", "Rust"]);
    }
}
