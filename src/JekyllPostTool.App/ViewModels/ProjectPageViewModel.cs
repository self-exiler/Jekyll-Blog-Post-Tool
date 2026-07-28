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

    [ObservableProperty]
    private bool _postsDirectoryExists;

    [ObservableProperty]
    private string _validationMessage = string.Empty;

    [ObservableProperty]
    private bool _isValidating;

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

        RefreshFromContext();
    }

    [RelayCommand]
    private async Task SelectFolderAsync()
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
    private async Task CreatePostsDirectoryAsync()
    {
        var project = _projectContext.CurrentProject;
        if (project is null)
        {
            await _dialogService.ShowInfoAsync("未选择项目", "请先选择一个博客项目。");
            return;
        }

        Directory.CreateDirectory(project.PostsDirectory);
        RefreshFromContext();
    }

    [RelayCommand]
    private async Task ValidateAsync()
    {
        IsValidating = true;
        await Task.Delay(100);
        RefreshFromContext();
        IsValidating = false;
    }

    private void RefreshFromContext()
    {
        var project = _projectContext.CurrentProject;
        if (project is null)
        {
            ProjectPath = "未选择项目";
            PostsDirectoryExists = false;
            ValidationMessage = "尚未选择博客项目";
            return;
        }

        ProjectPath = project.Path;
        PostsDirectoryExists = Directory.Exists(project.PostsDirectory);
        ValidationMessage = PostsDirectoryExists
            ? "_posts/ 目录已存在"
            : "_posts/ 目录不存在，需要创建";
    }
}
