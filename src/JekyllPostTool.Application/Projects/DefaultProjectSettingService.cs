namespace JekyllPostTool.Application.Projects;

/// <summary>
/// 管理默认项目路径的读写，直接持久化到 settings.json。
/// </summary>
public sealed class DefaultProjectSettingService
{
    private readonly JsonFileStore _store;

    public DefaultProjectSettingService(string appDataDirectory)
    {
        _store = new JsonFileStore(appDataDirectory, "settings.json");
    }

    public async Task<string?> GetAsync(CancellationToken cancellationToken = default)
    {
        var settings = await _store.ReadAsync<DefaultProjectSettings>(cancellationToken);
        return settings?.DefaultProjectPath;
    }

    public Task SetAsync(string? path, CancellationToken cancellationToken = default)
        => _store.WriteAsync(new DefaultProjectSettings(path), cancellationToken);

    private sealed record DefaultProjectSettings(string? DefaultProjectPath);
}
