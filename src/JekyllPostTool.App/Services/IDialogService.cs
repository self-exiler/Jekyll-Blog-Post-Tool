using JekyllPostTool.Application.Posts;

namespace JekyllPostTool_App.Services;

/// <summary>
/// 对话框服务抽象。
/// </summary>
public interface IDialogService
{
    Task ShowInfoAsync(string title, string message);

    Task<bool> ShowConfirmAsync(string title, string message, string primaryButtonText = "确认", string closeButtonText = "取消");

    Task<string?> ShowTextInputAsync(string title, string placeholder, string defaultText = "");

    /// <summary>
    /// 显示文件名冲突处理选项，返回用户选择的处理方式；取消返回 null。
    /// </summary>
    Task<ConflictResolutionKind?> ShowConflictResolutionAsync(string fileName, IEnumerable<ConflictResolution> resolutions);
}
