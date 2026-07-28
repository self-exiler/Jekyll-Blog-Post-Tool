using JekyllPostTool.Domain.Settings;

namespace JekyllPostTool.Application.Projects;

/// <summary>
/// 管理默认项目路径的读写。
/// </summary>
public sealed class DefaultProjectSettingService
{
    private readonly IAppSettingsRepository _repository;

    public DefaultProjectSettingService(IAppSettingsRepository repository)
    {
        _repository = repository;
    }

    public async Task<string?> GetAsync(CancellationToken cancellationToken = default)
    {
        var settings = await _repository.LoadAsync(cancellationToken);
        return settings.DefaultProjectPath;
    }

    public async Task SetAsync(string? path, CancellationToken cancellationToken = default)
    {
        var settings = new AppSettings(path);
        await _repository.SaveAsync(settings, cancellationToken);
    }

    public async Task ClearAsync(CancellationToken cancellationToken = default)
    {
        await _repository.SaveAsync(new AppSettings(null), cancellationToken);
    }
}
