namespace JekyllPostTool.Domain.Settings;

/// <summary>
/// 工具设置仓储接口。
/// </summary>
public interface IAppSettingsRepository
{
    Task<AppSettings> LoadAsync(CancellationToken cancellationToken = default);

    Task SaveAsync(AppSettings settings, CancellationToken cancellationToken = default);
}
