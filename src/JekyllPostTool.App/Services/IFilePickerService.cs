namespace JekyllPostTool_App.Services;

/// <summary>
/// 文件选择器服务抽象。
/// </summary>
public interface IFilePickerService
{
    Task<string?> PickFolderAsync(string? suggestedStartLocation = null);

    Task<string?> PickFileAsync(string? suggestedStartLocation = null);
}
