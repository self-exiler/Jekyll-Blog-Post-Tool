using System.Text.Json;
using JekyllPostTool.Domain.Settings;

namespace JekyllPostTool.Infrastructure.Persistence;

/// <summary>
/// 以 JSON 持久化工具级设置。
/// </summary>
public sealed class JsonAppSettingsRepository : IAppSettingsRepository
{
    private readonly string _settingsDirectory;
    private readonly string _settingsFilePath;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true
    };

    public JsonAppSettingsRepository(string settingsDirectory)
    {
        _settingsDirectory = settingsDirectory;
        _settingsFilePath = Path.Combine(settingsDirectory, "settings.json");
    }

    public Task<AppSettings> LoadAsync(CancellationToken cancellationToken = default)
    {
        if (!File.Exists(_settingsFilePath))
        {
            return Task.FromResult(new AppSettings(null));
        }

        var json = File.ReadAllText(_settingsFilePath);
        var settings = JsonSerializer.Deserialize<AppSettingsDto>(json, JsonOptions);
        return Task.FromResult(new AppSettings(settings?.DefaultProjectPath));
    }

    public Task SaveAsync(AppSettings settings, CancellationToken cancellationToken = default)
    {
        Directory.CreateDirectory(_settingsDirectory);

        var dto = new AppSettingsDto(settings.DefaultProjectPath);
        var json = JsonSerializer.Serialize(dto, JsonOptions);
        File.WriteAllText(_settingsFilePath, json);

        return Task.CompletedTask;
    }

    private sealed record AppSettingsDto(string? DefaultProjectPath);
}
