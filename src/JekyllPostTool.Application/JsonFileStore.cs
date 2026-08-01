using System.Text.Json;

namespace JekyllPostTool.Application;

/// <summary>
/// 通用 JSON 文件读写，消除 AiSettingsService 与 DefaultProjectSettingService 的重复 I/O 逻辑。
/// </summary>
public sealed class JsonFileStore
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true
    };

    private readonly string _filePath;

    public JsonFileStore(string appDataDirectory, string fileName)
    {
        _filePath = Path.Combine(appDataDirectory, fileName);
    }

    public async Task<T?> ReadAsync<T>(CancellationToken cancellationToken = default)
    {
        if (!File.Exists(_filePath))
        {
            return default;
        }

        var json = await File.ReadAllTextAsync(_filePath, cancellationToken);
        return JsonSerializer.Deserialize<T>(json, JsonOptions);
    }

    public async Task WriteAsync<T>(T data, CancellationToken cancellationToken = default)
    {
        var directory = Path.GetDirectoryName(_filePath);
        if (!string.IsNullOrEmpty(directory))
        {
            Directory.CreateDirectory(directory);
        }

        var json = JsonSerializer.Serialize(data, JsonOptions);
        await File.WriteAllTextAsync(_filePath, json, cancellationToken);
    }
}
