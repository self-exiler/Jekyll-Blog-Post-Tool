namespace JekyllPostTool.Application.Ai;

/// <summary>
/// 管理 AI 配置的读写，直接持久化到 ai.json（FR-7.1）。
/// </summary>
public sealed class AiSettingsService
{
    private readonly JsonFileStore _store;

    public AiSettingsService(string appDataDirectory)
    {
        _store = new JsonFileStore(appDataDirectory, "ai.json");
    }

    public async Task<AiSettings> GetAsync(CancellationToken cancellationToken = default)
    {
        return await _store.ReadAsync<AiSettings>(cancellationToken) ?? new AiSettings();
    }

    public Task SetAsync(AiSettings settings, CancellationToken cancellationToken = default)
        => _store.WriteAsync(settings, cancellationToken);
}
