using System.Diagnostics;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using JekyllPostTool.Application.Projects;
using JekyllPostTool.Domain.Projects;
using JekyllPostTool_App.Services;

namespace JekyllPostTool_App.ViewModels;

/// <summary>
/// 项目页视图模型。
/// </summary>
public sealed partial class ProjectPageViewModel : ObservableObject
{
    private readonly IProjectContext _projectContext;
    private readonly DefaultProjectSettingService _settingService;
    private readonly IFilePickerService _filePickerService;
    private readonly IDialogService _dialogService;

    [ObservableProperty]
    private string _projectPath = "未选择项目";

    public ProjectPageViewModel(
        IProjectContext projectContext,
        DefaultProjectSettingService settingService,
        IFilePickerService filePickerService,
        IDialogService dialogService)
    {
        _projectContext = projectContext;
        _settingService = settingService;
        _filePickerService = filePickerService;
        _dialogService = dialogService;

        _projectContext.CurrentProjectChanged += OnCurrentProjectChanged;
        RefreshFromContext();
    }

    private void OnCurrentProjectChanged(object? sender, EventArgs e)
    {
        RefreshFromContext();
    }

    [RelayCommand]
    private async Task SelectProjectPathAsync()
    {
        var path = await _filePickerService.PickFolderAsync();
        if (string.IsNullOrWhiteSpace(path))
        {
            return;
        }

        if (!Directory.Exists(path))
        {
            await _dialogService.ShowInfoAsync("路径无效", "所选文件夹不存在。");
            return;
        }

        var project = new BlogProject(path);
        _projectContext.CurrentProject = project;
        await _settingService.SetAsync(path);

        RefreshFromContext();
    }

    [RelayCommand]
    private async Task OpenInExplorerAsync()
    {
        var project = _projectContext.CurrentProject;
        if (project is null)
        {
            await _dialogService.ShowInfoAsync("未选择项目", "请先选择一个博客项目。");
            return;
        }

        try
        {
            Process.Start(new ProcessStartInfo("explorer.exe", project.Path) { UseShellExecute = true });
        }
        catch (Exception ex)
        {
            await _dialogService.ShowInfoAsync("打开失败", $"无法在资源管理器中打开项目：{ex.Message}");
        }
    }

    [RelayCommand]
    private async Task OpenInVsCodeAsync()
    {
        var project = _projectContext.CurrentProject;
        if (project is null)
        {
            await _dialogService.ShowInfoAsync("未选择项目", "请先选择一个博客项目。");
            return;
        }

        try
        {
            Process.Start(new ProcessStartInfo("code", project.Path) { UseShellExecute = true });
        }
        catch (Exception ex)
        {
            await _dialogService.ShowInfoAsync(
                "打开失败",
                $"无法在 VS Code 中打开项目。请确认 VS Code 已安装且 `code` 命令在 PATH 中。{Environment.NewLine}错误：{ex.Message}");
        }
    }

    private void RefreshFromContext()
    {
        var project = _projectContext.CurrentProject;
        ProjectPath = project is null ? "未选择项目" : project.Path;
    }
}
