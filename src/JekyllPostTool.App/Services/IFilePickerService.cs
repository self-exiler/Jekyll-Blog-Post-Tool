namespace JekyllPostTool_App.Services;

/// <summary>
/// 文件选择器服务抽象。
/// </summary>
public interface IFilePickerService
{
    Task<string?> PickFolderAsync();

    Task<string?> PickFileAsync();

    /// <summary>
    /// 多选文件（FR-6.2）。<paramref name="fileTypes"/> 为扩展名（含点，如 ".jpg"）。
    /// 返回 null 表示用户取消。
    /// </summary>
    Task<IReadOnlyList<string>?> PickFilesAsync(IEnumerable<string> fileTypes);
}
