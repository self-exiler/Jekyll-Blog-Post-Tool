using JekyllPostTool.Application.Ai;

namespace JekyllPostTool.Application.Tests;

/// <summary>
/// AiSettingsService 单元测试，验证 ai.json 的读写与默认值。
/// </summary>
public sealed class AiSettingsServiceTests : IDisposable
{
    private readonly string _tempDir = Path.Combine(Path.GetTempPath(), "AiSettingsTests-" + Guid.NewGuid().ToString("N"));

    public AiSettingsServiceTests()
    {
        Directory.CreateDirectory(_tempDir);
    }

    public void Dispose()
    {
        try { Directory.Delete(_tempDir, recursive: true); } catch { /* 忽略 */ }
    }

    [Fact]
    public async Task GetAsync_NoFile_ReturnsEmptySettings()
    {
        var service = new AiSettingsService(_tempDir);

        var settings = await service.GetAsync();

        Assert.Equal(string.Empty, settings.BaseUrl);
        Assert.Equal(string.Empty, settings.ApiKey);
        Assert.Equal(string.Empty, settings.Model);
        Assert.False(settings.IsConfigured);
    }

    [Fact]
    public async Task SetAsync_PersistsAndRoundTrips()
    {
        var service = new AiSettingsService(_tempDir);
        var settings = new AiSettings
        {
            BaseUrl = "https://api.openai.com/v1",
            ApiKey = "sk-test",
            Model = "gpt-4o-mini"
        };

        await service.SetAsync(settings);

        var loaded = await service.GetAsync();
        Assert.Equal(settings.BaseUrl, loaded.BaseUrl);
        Assert.Equal(settings.ApiKey, loaded.ApiKey);
        Assert.Equal(settings.Model, loaded.Model);
        Assert.True(loaded.IsConfigured);
    }

    [Fact]
    public async Task SetAsync_CreatesDirectoryIfMissing()
    {
        var nestedDir = Path.Combine(_tempDir, "nested", "config");
        var service = new AiSettingsService(nestedDir);

        await service.SetAsync(new AiSettings { BaseUrl = "https://x", ApiKey = "k", Model = "m" });

        Assert.True(File.Exists(Path.Combine(nestedDir, "ai.json")));
    }

    [Fact]
    public async Task SetAsync_OverwritesPreviousValue()
    {
        var service = new AiSettingsService(_tempDir);
        await service.SetAsync(new AiSettings { BaseUrl = "https://old", ApiKey = "k1", Model = "m1" });

        await service.SetAsync(new AiSettings { BaseUrl = "https://new", ApiKey = "k2", Model = "m2" });

        var loaded = await service.GetAsync();
        Assert.Equal("https://new", loaded.BaseUrl);
        Assert.Equal("k2", loaded.ApiKey);
    }

    [Fact]
    public void IsConfigured_FalseWhenAnyFieldBlank()
    {
        Assert.False(new AiSettings { BaseUrl = "x", ApiKey = "", Model = "m" }.IsConfigured);
        Assert.False(new AiSettings { BaseUrl = "", ApiKey = "k", Model = "m" }.IsConfigured);
        Assert.False(new AiSettings { BaseUrl = "  ", ApiKey = "k", Model = "m" }.IsConfigured);
        Assert.False(new AiSettings().IsConfigured);
    }

    [Fact]
    public void IsConfigured_TrueWhenAllFieldsPresent()
    {
        Assert.True(new AiSettings { BaseUrl = "x", ApiKey = "k", Model = "m" }.IsConfigured);
    }
}
