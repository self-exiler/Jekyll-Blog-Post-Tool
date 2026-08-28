using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;
using System.Text.Json.Serialization;
using JekyllPostTool.Application.Ai;

namespace JekyllPostTool.Infrastructure.Ai;

/// <summary>
/// 基于 OpenAI 兼容 Chat Completions API 的 AI 关键字提取实现（FR-7.2~7.3）。
/// 每次调用读取最新配置，以便用户在高级功能页修改后立即生效。
/// </summary>
public sealed class OpenAiService
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower
    };

    private readonly HttpClient _httpClient;
    private readonly AiSettingsService _settingsService;

    public OpenAiService(HttpClient httpClient, AiSettingsService settingsService)
    {
        _httpClient = httpClient;
        _settingsService = settingsService;
    }

    public async Task<IReadOnlyList<string>> ExtractKeywordsAsync(
        string body,
        int maxCount = 5,
        CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(body);

        var settings = await _settingsService.GetAsync(cancellationToken);
        if (!settings.IsConfigured)
        {
            throw new InvalidOperationException("尚未配置 AI API（Base URL / API Key / 模型）。");
        }

        var endpoint = BuildEndpoint(settings.BaseUrl);

        using var request = new HttpRequestMessage(HttpMethod.Post, endpoint);
        request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", settings.ApiKey);

        var systemPrompt = $"你是一个关键字提取助手。从给定的 markdown 博文正文中提取最多 {maxCount} 个最能代表内容的关键字。"
            + "关键字应为简短的词或短语（中文或英文，跟随正文语言）。仅返回以英文逗号分隔的关键字列表，不要编号、不要解释、不要其他内容。";
        request.Content = JsonContent.Create(new ChatRequest
        {
            Model = settings.Model,
            Temperature = 0.2,
            Messages = new[]
            {
                new ChatMessage { Role = "system", Content = systemPrompt },
                new ChatMessage { Role = "user", Content = body }
            }
        }, options: JsonOptions);

        using var response = await _httpClient.SendAsync(request, cancellationToken);
        if (!response.IsSuccessStatusCode)
        {
            var error = await response.Content.ReadAsStringAsync(cancellationToken);
            throw new InvalidOperationException($"AI API 调用失败 ({(int)response.StatusCode} {response.StatusCode}): {TrimError(error)}");
        }

        var chatResponse = await response.Content.ReadFromJsonAsync<ChatResponse>(JsonOptions, cancellationToken);
        var content = chatResponse?.Choices?.ElementAtOrDefault(0)?.Message?.Content ?? string.Empty;

        return ParseKeywords(content, maxCount);
    }

    private static string BuildEndpoint(string baseUrl)
    {
        var trimmed = baseUrl.TrimEnd('/');
        // 若已包含 /chat/completions 直接使用；否则补全
        if (trimmed.EndsWith("/chat/completions", StringComparison.OrdinalIgnoreCase))
        {
            return trimmed;
        }

        // 路径段级判断是否已含版本段，避免 "/v1-proxy" 之类子串误判；否则补 /v1
        var hasVersionSegment = trimmed
            .Split('/', StringSplitOptions.RemoveEmptyEntries)
            .Contains("v1", StringComparer.OrdinalIgnoreCase);

        return hasVersionSegment
            ? trimmed + "/chat/completions"
            : trimmed + "/v1/chat/completions";
    }

    private static string TrimError(string error)
    {
        const int max = 500;
        return error.Length > max ? error[..max] + "..." : error;
    }

    private static IReadOnlyList<string> ParseKeywords(string content, int maxCount)
    {
        // 兼容中英文逗号、换行、顿号
        var tokens = content.Split([',', '，', '\n', '\r', '、'], StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        var keywords = new List<string>();
        // 大小写不敏感去重，保留首次出现的大小写
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var token in tokens)
        {
            // 去除可能的序号前缀（如 "1." "1、" "1:"）
            var cleaned = CleanToken(token);
            if (string.IsNullOrWhiteSpace(cleaned))
            {
                continue;
            }

            if (seen.Add(cleaned))
            {
                keywords.Add(cleaned);
                if (keywords.Count >= maxCount)
                {
                    break;
                }
            }
        }

        return keywords;
    }

    private static string CleanToken(string token)
    {
        // 去除前导序号：数字 + . 或 : 或 、
        var span = token.AsSpan().Trim();
        var i = 0;
        while (i < span.Length && char.IsDigit(span[i]))
        {
            i++;
        }

        if (i > 0 && i < span.Length && (span[i] == '.' || span[i] == ':' || span[i] == '、' || span[i] == ')'))
        {
            span = span[(i + 1)..];
        }

        return span.Trim(TrimChars).ToString();
    }

    private static readonly char[] TrimChars = { ' ', '\t', '"', '\'', '“', '”', '‘', '’' };

    private sealed class ChatRequest
    {
        public string Model { get; init; } = string.Empty;
        public double? Temperature { get; init; }
        public required IReadOnlyList<ChatMessage> Messages { get; init; }
    }

    private sealed class ChatMessage
    {
        public required string Role { get; init; }
        public required string Content { get; init; }
    }

    private sealed class ChatResponse
    {
        public IReadOnlyList<Choice>? Choices { get; init; }
    }

    private sealed class Choice
    {
        public ChatMessage? Message { get; init; }
    }
}
