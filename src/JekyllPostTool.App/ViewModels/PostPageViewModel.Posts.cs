using CommunityToolkit.Mvvm.Input;
using JekyllPostTool.Application.Posts;
using JekyllPostTool.Domain.Posts;

namespace JekyllPostTool_App.ViewModels;

/// <summary>
/// 博文 CRUD 操作 partial：新建、打开、保存、加载。冲突重试循环在 PostSaveUseCase 内，
/// 这里只负责把用户回答经 seam 传给用例并展示结果。
/// </summary>
public sealed partial class PostPageViewModel
{
    [RelayCommand]
    private void NewPost()
    {
        _originalFilePath = null;
        _originalContentHash = null;
        PageTitle = "博文";

        // FR-3.1：新建时 date 为空，不预填默认值
        ApplyFormState(PostFormState.Empty(DateTimeOffset.Now));
        Body = string.Empty;

        foreach (var author in AvailableAuthors)
        {
            author.IsSelected = false;
        }

        UpdateAuthorsDisplay();
        UpdatePreview();
        NotifyPostFileChanged();
    }

    [RelayCommand]
    private async Task OpenPostAsync()
    {
        var project = _projectContext.CurrentProject;
        if (project is null)
        {
            await _dialogService.ShowInfoAsync("未选择项目", "请先选择一个博客项目。");
            return;
        }

        var filePath = await _filePickerService.PickFileAsync();
        if (string.IsNullOrWhiteSpace(filePath))
        {
            return;
        }

        var postsDirectory = Path.GetFullPath(project.PostsDirectory);
        var selectedDirectory = Path.GetFullPath(Path.GetDirectoryName(filePath)!);
        if (!string.Equals(selectedDirectory, postsDirectory, StringComparison.OrdinalIgnoreCase))
        {
            await _dialogService.ShowInfoAsync("路径无效", "只能打开当前项目 _posts/ 目录下的 .md 文件。");
            return;
        }

        await LoadPostAsync(filePath);
    }

    [RelayCommand(CanExecute = nameof(CanSave))]
    private async Task SavePostAsync()
    {
        var project = _projectContext.CurrentProject;
        if (project is null)
        {
            await _dialogService.ShowInfoAsync("未选择项目", "请先选择一个博客项目。");
            return;
        }

        FrontMatter frontMatter;
        try
        {
            frontMatter = BuildFormState().ToFrontMatter();
        }
        catch (Exception ex)
        {
            await _dialogService.ShowInfoAsync("输入无效", ex.Message);
            return;
        }

        IsBusy = true;
        PostOperationResult result;
        try
        {
            result = await SaveViaUseCaseAsync(project, frontMatter, _originalContentHash);

            if (result.IsModifiedExternally)
            {
                var continueSave = await _dialogService.ShowConfirmAsync(
                    "文件已被外部修改",
                    "该博文在磁盘上已被其他程序修改。继续保存将覆盖外部修改。建议选择“取消并刷新”以加载最新内容。",
                    "继续保存",
                    "取消并刷新");

                if (!continueSave)
                {
                    if (_originalFilePath is not null)
                    {
                        await LoadPostAsync(_originalFilePath);
                    }

                    return;
                }

                // 覆盖外部修改：置空基线哈希后重存（冲突循环仍由用例处理）
                result = await SaveViaUseCaseAsync(project, frontMatter, originalContentHash: null);
            }
        }
        catch (Exception ex)
        {
            // 磁盘 IO / YAML 异常不再静默吞掉
            await _dialogService.ShowInfoAsync("保存失败", ex.Message);
            return;
        }
        finally
        {
            IsBusy = false;
        }

        if (!result.IsSuccess)
        {
            if (result.IsConflict)
            {
                // 用户已取消冲突处理或放弃覆盖
                return;
            }

            var message = string.Join(Environment.NewLine, result.Errors.Select(e => e.Message));
            await _dialogService.ShowInfoAsync("保存失败", message);
            return;
        }

        if (result.Warnings is { Count: > 0 } warnings)
        {
            await _dialogService.ShowInfoAsync("已保存（有警告）", string.Join(Environment.NewLine, warnings));
        }

        _originalFilePath = result.FilePath;
        _originalContentHash = await _saveUseCase.GetContentHashAsync(result.FilePath!);
        PageTitle = $"博文 - {Path.GetFileName(result.FilePath)}";
        NotifyPostFileChanged();

        await _dialogService.ShowInfoAsync("保存成功", $"博文已保存到 {Path.GetRelativePath(project.Path, result.FilePath!)}");
    }

    private bool CanSave() => !IsBusy;

    private async Task<PostOperationResult> SaveViaUseCaseAsync(
        JekyllPostTool.Domain.Projects.BlogProject project,
        FrontMatter frontMatter,
        string? originalContentHash)
    {
        return await _saveUseCase.SaveAsync(
            project,
            frontMatter,
            Body,
            new SavePrompts(ShowConflictDialogAsync, ConfirmOverwriteAsync),
            _originalFilePath,
            originalContentHash);
    }

    private Task<ConflictResolutionKind?> ShowConflictDialogAsync(ConflictResult conflict) =>
        _dialogService.ShowConflictResolutionAsync(Path.GetFileName(conflict.FilePath), conflict.AutoSuffix);

    private Task<bool> ConfirmOverwriteAsync(string fileName) =>
        _dialogService.ShowConfirmAsync("确认覆盖", $"确定覆盖现有文件 {fileName} 吗？");

    private async Task LoadPostAsync(string filePath)
    {
        PostSaveUseCase.LoadedPost loaded;
        try
        {
            var read = await _saveUseCase.LoadAsync(filePath);
            if (read is null)
            {
                await _dialogService.ShowInfoAsync("打开失败", "无法读取博文文件。");
                return;
            }

            loaded = read;
        }
        catch (Exception ex)
        {
            await _dialogService.ShowInfoAsync("打开失败", ex.Message);
            return;
        }

        _originalFilePath = loaded.Post.FilePath;
        // 单次读盘：展示内容与基线哈希同源，杜绝双重读取之间的竞态
        _originalContentHash = loaded.ContentHash;
        PageTitle = $"博文 - {loaded.Post.FileName}";

        await ApplyFormStateAndRefreshAuthorsAsync(
            PostFormState.FromFrontMatter(loaded.Post.FrontMatter),
            loaded.Post.FrontMatter.Authors);
        Body = loaded.Post.Body;

        UpdateAuthorsDisplay();
        UpdatePreview();

        NotifyPostFileChanged();
    }
}
