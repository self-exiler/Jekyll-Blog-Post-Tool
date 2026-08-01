namespace JekyllPostTool.Application.Ai;

/// <summary>
/// OpenAI 兼容 API 的配置（FR-7.1）。
/// </summary>
public sealed record AiSettings
{
    /// <summary>API 基址，如 <c>https://api.openai.com/v1</c>。</summary>
    public string BaseUrl { get; init; } = string.Empty;

    /// <summary>API Key（Bearer 令牌）。</summary>
    public string ApiKey { get; init; } = string.Empty;

    /// <summary>模型名，如 <c>gpt-4o-mini</c>。</summary>
    public string Model { get; init; } = string.Empty;

    /// <summary>是否已填写完整可调用。</summary>
    public bool IsConfigured =>
        !string.IsNullOrWhiteSpace(BaseUrl)
        && !string.IsNullOrWhiteSpace(ApiKey)
        && !string.IsNullOrWhiteSpace(Model);
}
