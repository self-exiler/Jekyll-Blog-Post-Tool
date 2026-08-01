using System.Net;
using System.Text;
using JekyllPostTool.Application.Ai;
using JekyllPostTool.Infrastructure.Ai;

namespace JekyllPostTool.Infrastructure.Tests;

/// <summary>
/// OpenAiService 单元测试，使用 stub HttpMessageHandler 避免真实网络调用。
/// </summary>
public sealed class OpenAiServiceTests : IDisposable
{
    private readonly string _tempDir = Path.Combine(Path.GetTempPath(), "OpenAiServiceTests-" + Guid.NewGuid().ToString("N"));

    public OpenAiServiceTests()
    {
        Directory.CreateDirectory(_tempDir);
    }

    public void Dispose()
    {
        try { Directory.Delete(_tempDir, recursive: true); } catch { /* 忽略 */ }
    }

    [Fact]
    public async Task ExtractKeywordsAsync_ParsesCommaSeparatedResponse()
    {
        var handler = new StubHandler(
            HttpStatusCode.OK,
            """{"choices":[{"message":{"role":"assistant","content":"Jekyll,Blog,Markdown,Automation,Tags"}}]}""");
        var service = CreateService(handler, configured: true);

        var keywords = await service.ExtractKeywordsAsync("正文内容");

        Assert.Equal(new[] { "Jekyll", "Blog", "Markdown", "Automation", "Tags" }, keywords);
    }

    [Fact]
    public async Task ExtractKeywordsAsync_HandlesNumberedList()
    {
        var handler = new StubHandler(
            HttpStatusCode.OK,
            "{\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"1. 博客\\n2. 写作\\n3. 工具\"}}]}");
        var service = CreateService(handler, configured: true);

        var keywords = await service.ExtractKeywordsAsync("正文");

        Assert.Equal(new[] { "博客", "写作", "工具" }, keywords);
    }

    [Fact]
    public async Task ExtractKeywordsAsync_DeduplicatesKeywords()
    {
        var handler = new StubHandler(
            HttpStatusCode.OK,
            "{\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"Jekyll, jekyll, Blog, blog\"}}]}");
        var service = CreateService(handler, configured: true);

        var keywords = await service.ExtractKeywordsAsync("正文");

        Assert.Equal(new[] { "Jekyll", "Blog" }, keywords);
    }

    [Fact]
    public async Task ExtractKeywordsAsync_RespectsMaxCount()
    {
        var handler = new StubHandler(
            HttpStatusCode.OK,
            """{"choices":[{"message":{"role":"assistant","content":"a,b,c,d,e,f,g"}}]}""");
        var service = CreateService(handler, configured: true);

        var keywords = await service.ExtractKeywordsAsync("正文", maxCount: 3);

        Assert.Equal(3, keywords.Count);
        Assert.Equal(new[] { "a", "b", "c" }, keywords);
    }

    [Fact]
    public async Task ExtractKeywordsAsync_HandlesChineseCommaAndNewlines()
    {
        var handler = new StubHandler(
            HttpStatusCode.OK,
            "{\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"关键字1，关键字2、关键字3\"}}]}");
        var service = CreateService(handler, configured: true);

        var keywords = await service.ExtractKeywordsAsync("正文");

        Assert.Equal(new[] { "关键字1", "关键字2", "关键字3" }, keywords);
    }

    [Fact]
    public async Task ExtractKeywordsAsync_ThrowsWhenNotConfigured()
    {
        var handler = new StubHandler(HttpStatusCode.OK, "{}");
        var service = CreateService(handler, configured: false);

        await Assert.ThrowsAsync<InvalidOperationException>(() =>
            service.ExtractKeywordsAsync("正文"));
    }

    [Fact]
    public async Task ExtractKeywordsAsync_ThrowsOnNonSuccessStatusCode()
    {
        var handler = new StubHandler(HttpStatusCode.Unauthorized, "invalid api key");
        var service = CreateService(handler, configured: true);

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(() =>
            service.ExtractKeywordsAsync("正文"));
        Assert.Contains("401", ex.Message);
    }

    [Fact]
    public async Task ExtractKeywordsAsync_ReturnsEmpty_WhenResponseHasNoChoices()
    {
        var handler = new StubHandler(HttpStatusCode.OK, """{"choices":[]}""");
        var service = CreateService(handler, configured: true);

        var keywords = await service.ExtractKeywordsAsync("正文");

        Assert.Empty(keywords);
    }

    [Fact]
    public async Task ExtractKeywordsAsync_SendsBearerToken()
    {
        var handler = new StubHandler(
            HttpStatusCode.OK,
            """{"choices":[{"message":{"role":"assistant","content":"kw"}}]}""");
        var service = CreateService(handler, configured: true, apiKey: "sk-secret");

        await service.ExtractKeywordsAsync("正文");

        Assert.Equal("Bearer", handler.LastRequest!.Headers.Authorization!.Scheme);
        Assert.Equal("sk-secret", handler.LastRequest.Headers.Authorization.Parameter);
    }

    [Theory]
    [InlineData("https://api.openai.com/v1", "https://api.openai.com/v1/chat/completions")]
    [InlineData("https://api.openai.com/v1/", "https://api.openai.com/v1/chat/completions")]
    [InlineData("https://proxy.example.com/v1", "https://proxy.example.com/v1/chat/completions")]
    [InlineData("https://custom.example.com", "https://custom.example.com/v1/chat/completions")]
    [InlineData("https://custom.example.com/v1/chat/completions", "https://custom.example.com/v1/chat/completions")]
    public async Task ExtractKeywordsAsync_BuildsCorrectEndpoint(string baseUrl, string expected)
    {
        var handler = new StubHandler(
            HttpStatusCode.OK,
            """{"choices":[{"message":{"role":"assistant","content":"kw"}}]}""");
        var service = CreateService(handler, configured: true, baseUrl: baseUrl);

        await service.ExtractKeywordsAsync("正文");

        Assert.Equal(expected, handler.LastRequest!.RequestUri!.ToString());
    }

    private OpenAiService CreateService(
        StubHandler handler,
        bool configured,
        string baseUrl = "https://api.openai.com/v1",
        string apiKey = "sk-test",
        string model = "gpt-4o-mini")
    {
        var settingsService = new AiSettingsService(_tempDir);
        settingsService.SetAsync(configured
            ? new AiSettings { BaseUrl = baseUrl, ApiKey = apiKey, Model = model }
            : new AiSettings()).Wait();
        return new OpenAiService(new HttpClient(handler), settingsService);
    }

    private sealed class StubHandler : HttpMessageHandler
    {
        private readonly HttpStatusCode _status;
        private readonly string _content;

        public HttpRequestMessage? LastRequest { get; private set; }

        public StubHandler(HttpStatusCode status, string content)
        {
            _status = status;
            _content = content;
        }

        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
        {
            LastRequest = request;
            return Task.FromResult(new HttpResponseMessage(_status)
            {
                Content = new StringContent(_content, Encoding.UTF8, "application/json")
            });
        }
    }
}
