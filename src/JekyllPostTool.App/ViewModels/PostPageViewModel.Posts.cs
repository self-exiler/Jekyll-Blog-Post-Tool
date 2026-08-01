using CommunityToolkit.Mvvm.Input;
using JekyllPostTool.Application.Posts;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;
using JekyllPostTool_App.Services;

namespace JekyllPostTool_App.ViewModels;

/// <summary>
/// 博文 CRUD 操作 partial：新建、打开、保存、加载、冲突处理。
/// </summary>
public sealed partial class PostPageViewModel
{
    [RelayCommand]
    private void NewPost()
    {
        _originalFilePath = null;
        _originalContentHash = null;
        PageTitle = "博文";

        Title = string.Empty;
        // FR-3.1：新建时 date 为空，不预填默认值
        SelectedDate = null;
        SelectedTime = DateTimeOffset.Now.TimeOfDay;
        SelectedTimeZone = TimeZoneFormatter.Format(DateTimeOffset.Now.Offset);
        Category1 = string.Empty;
        Category2 = string.Empty;
        Tags = string.Empty;
        Description = string.Empty;
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

    [RelayCommand]
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
            frontMatter = BuildFrontMatter();
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
            if (_originalFilePath is null)
            {
                result = await _postCreateUseCase.CreateAsync(project, frontMatter, Body);
                result = await ResolveConflictAsync(result, project, frontMatter, null);
            }
            else
            {
                result = await _postEditUseCase.UpdateAsync(project, _originalFilePath, frontMatter, _originalContentHash);
                result = await ResolveConflictAsync(result, project, frontMatter, _originalFilePath);
            }
        }
        finally
        {
            IsBusy = false;
        }

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

            result = _originalFilePath is null
                ? await _postCreateUseCase.CreateAsync(project, frontMatter, Body)
                : await _postEditUseCase.UpdateAsync(project, _originalFilePath, frontMatter, null);
        }

        if (!result.IsSuccess)
        {
            if (result.IsConflict && result.Conflict is not null)
            {
                // 用户已取消冲突处理
                return;
            }

            var message = string.Join(Environment.NewLine, result.Errors.Select(e => e.Message));
            await _dialogService.ShowInfoAsync("保存失败", message);
            return;
        }

        _originalFilePath = result.FilePath;
        _originalContentHash = await _postEditUseCase.ComputeContentHashAsync(result.FilePath);
        PageTitle = $"博文 - {Path.GetFileName(result.FilePath)}";
        NotifyPostFileChanged();

        await _dialogService.ShowInfoAsync("保存成功", $"博文已保存到 {Path.GetRelativePath(project.Path, result.FilePath)}");
    }

    private async Task LoadPostAsync(string filePath)
    {
        var post = await _postEditUseCase.LoadAsync(filePath);
        if (post is null)
        {
            await _dialogService.ShowInfoAsync("打开失败", "无法读取博文文件。");
            return;
        }

        _originalFilePath = post.FilePath;
        _originalContentHash = await _postEditUseCase.ComputeContentHashAsync(post.FilePath);
        PageTitle = $"博文 - {post.FileName}";

        Title = post.FrontMatter.Title;
        if (post.FrontMatter.Date.HasValue)
        {
            var date = post.FrontMatter.Date.Value;
            SelectedDate = date;
            SelectedTime = date.DateTime.TimeOfDay;
            SelectedTimeZone = TimeZoneFormatter.Format(date.Offset);
        }
        else
        {
            SelectedDate = null;
        }

        Category1 = post.FrontMatter.Categories.ElementAtOrDefault(0)?.Value ?? string.Empty;
        Category2 = post.FrontMatter.Categories.ElementAtOrDefault(1)?.Value ?? string.Empty;
        Tags = string.Join(", ", post.FrontMatter.Tags.Select(t => t.Value));
        Description = post.FrontMatter.Description ?? string.Empty;
        Body = post.Body;

        await LoadAuthorsAsync();

        var selectedIds = new HashSet<string>(post.FrontMatter.Authors, StringComparer.Ordinal);
        foreach (var option in AvailableAuthors)
        {
            option.IsSelected = selectedIds.Contains(option.Id);
        }

        UpdateAuthorsDisplay();
        UpdatePreview();

        NotifyPostFileChanged();
    }

    private async Task<PostOperationResult> ResolveConflictAsync(
        PostOperationResult result,
        BlogProject project,
        FrontMatter frontMatter,
        string? originalFilePath)
    {
        while (result.IsConflict && result.Conflict is not null)
        {
            var kind = await _dialogService.ShowConflictResolutionAsync(
                Path.GetFileName(result.Conflict.FilePath),
                result.Conflict.Resolutions);

            if (kind is null)
            {
                return result;
            }

            if (kind == ConflictResolutionKind.Overwrite)
            {
                var confirmed = await _dialogService.ShowConfirmAsync("确认覆盖", "确定覆盖现有文件吗？");
                if (!confirmed)
                {
                    return result;
                }
            }

            result = originalFilePath is null
                ? await _postCreateUseCase.CreateAsync(project, frontMatter, Body, kind)
                : await _postEditUseCase.UpdateAsync(project, originalFilePath, frontMatter, _originalContentHash, kind);
        }

        return result;
    }
}
