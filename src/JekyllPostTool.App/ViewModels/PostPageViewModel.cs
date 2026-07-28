using System.Collections.ObjectModel;
using System.Globalization;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using JekyllPostTool.Application.Authors;
using JekyllPostTool.Application.Posts;
using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;
using JekyllPostTool.Infrastructure.Import;
using JekyllPostTool.Infrastructure.Yaml;
using JekyllPostTool_App.Services;

namespace JekyllPostTool_App.ViewModels;

/// <summary>
/// 博文编辑页视图模型。
/// </summary>
public sealed partial class PostPageViewModel : ObservableObject
{
    private readonly IProjectContext _projectContext;
    private readonly PostCreateUseCase _postCreateUseCase;
    private readonly PostEditUseCase _postEditUseCase;
    private readonly AuthorCrudUseCase _authorUseCase;
    private readonly IFilePickerService _filePickerService;
    private readonly IDialogService _dialogService;
    private readonly MarkdownBodyImporter _bodyImporter;

    private string? _originalFilePath;
    private string? _originalContentHash;

    [ObservableProperty]
    private string _title = string.Empty;

    [ObservableProperty]
    private DateTimeOffset _selectedDate = DateTimeOffset.Now;

    [ObservableProperty]
    private TimeSpan _selectedTime = DateTimeOffset.Now.TimeOfDay;

    [ObservableProperty]
    private string _selectedTimeZone = FormatOffset(DateTimeOffset.Now.Offset);

    [ObservableProperty]
    private string _category1 = string.Empty;

    [ObservableProperty]
    private string _category2 = string.Empty;

    [ObservableProperty]
    private string _tags = string.Empty;

    [ObservableProperty]
    private string _description = string.Empty;

    [ObservableProperty]
    private string _body = string.Empty;

    [ObservableProperty]
    private string _fileNamePreview = string.Empty;

    [ObservableProperty]
    private string _frontMatterPreview = string.Empty;

    [ObservableProperty]
    private bool _isBusy;

    [ObservableProperty]
    private string _pageTitle = "博文";

    public ObservableCollection<AuthorOption> AvailableAuthors { get; } = new();

    public IReadOnlyList<string> TimeZoneOptions { get; } = BuildTimeZoneOptions();

    public PostPageViewModel(
        IProjectContext projectContext,
        PostCreateUseCase postCreateUseCase,
        PostEditUseCase postEditUseCase,
        AuthorCrudUseCase authorUseCase,
        IFilePickerService filePickerService,
        IDialogService dialogService,
        MarkdownBodyImporter bodyImporter)
    {
        _projectContext = projectContext;
        _postCreateUseCase = postCreateUseCase;
        _postEditUseCase = postEditUseCase;
        _authorUseCase = authorUseCase;
        _filePickerService = filePickerService;
        _dialogService = dialogService;
        _bodyImporter = bodyImporter;

        _projectContext.CurrentProjectChanged += OnCurrentProjectChanged;
    }

    protected override void OnPropertyChanged(global::System.ComponentModel.PropertyChangedEventArgs e)
    {
        base.OnPropertyChanged(e);
        if (e.PropertyName is nameof(Title)
            or nameof(SelectedDate)
            or nameof(SelectedTime)
            or nameof(SelectedTimeZone)
            or nameof(Category1)
            or nameof(Category2)
            or nameof(Tags)
            or nameof(Description))
        {
            UpdatePreview();
        }
    }

    [RelayCommand]
    public async Task LoadAuthorsAsync()
    {
        AvailableAuthors.Clear();

        if (_projectContext.CurrentProject is null)
        {
            return;
        }

        var authors = await _authorUseCase.ListAsync();
        foreach (var author in authors)
        {
            AvailableAuthors.Add(new AuthorOption(author, UpdatePreview));
        }

        UpdatePreview();
    }

    [RelayCommand]
    private void NewPost()
    {
        _originalFilePath = null;
        _originalContentHash = null;
        PageTitle = "博文";

        Title = string.Empty;
        SelectedDate = DateTimeOffset.Now;
        SelectedTime = DateTimeOffset.Now.TimeOfDay;
        SelectedTimeZone = FormatOffset(DateTimeOffset.Now.Offset);
        Category1 = string.Empty;
        Category2 = string.Empty;
        Tags = string.Empty;
        Description = string.Empty;
        Body = string.Empty;

        foreach (var author in AvailableAuthors)
        {
            author.IsSelected = false;
        }

        UpdatePreview();
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

        var filePath = await _filePickerService.PickFileAsync(project.PostsDirectory);
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

        await _dialogService.ShowInfoAsync("保存成功", $"博文已保存到 {Path.GetRelativePath(project.Path, result.FilePath)}");
    }

    [RelayCommand]
    private async Task ImportBodyAsync()
    {
        var filePath = await _filePickerService.PickFileAsync();
        if (string.IsNullOrWhiteSpace(filePath))
        {
            return;
        }

        Body = await _bodyImporter.ImportAsync(filePath);
    }

    private async Task LoadPostAsync(string filePath)
    {
        var loadResult = await _postEditUseCase.LoadAsync(filePath);
        if (!loadResult.IsSuccess || loadResult.Post is null)
        {
            await _dialogService.ShowInfoAsync("打开失败", "无法读取博文文件。");
            return;
        }

        var post = loadResult.Post;
        _originalFilePath = post.FilePath;
        _originalContentHash = await _postEditUseCase.ComputeContentHashAsync(post.FilePath);
        PageTitle = $"博文 - {post.FileName}";

        Title = post.FrontMatter.Title;
        if (post.FrontMatter.Date.HasValue)
        {
            var date = post.FrontMatter.Date.Value;
            SelectedDate = date;
            SelectedTime = date.DateTime.TimeOfDay;
            SelectedTimeZone = FormatOffset(date.Offset);
        }
        else
        {
            SelectedDate = DateTimeOffset.Now;
            SelectedTime = DateTimeOffset.Now.TimeOfDay;
            SelectedTimeZone = FormatOffset(DateTimeOffset.Now.Offset);
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

        UpdatePreview();
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

    private FrontMatter BuildFrontMatter()
    {
        var categories = new List<Category>();
        if (!string.IsNullOrWhiteSpace(Category1))
        {
            categories.Add(new Category(Category1));
        }

        if (!string.IsNullOrWhiteSpace(Category2))
        {
            categories.Add(new Category(Category2));
        }

        var tags = Tags.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .Where(t => !string.IsNullOrWhiteSpace(t))
            .Select(t => new Tag(t))
            .ToList();

        var selectedAuthors = AvailableAuthors
            .Where(a => a.IsSelected)
            .Select(a => a.Id)
            .ToList();

        var date = ComputeDate();

        return new FrontMatter
        {
            Title = Title,
            Date = date,
            Categories = categories,
            Tags = tags,
            Authors = selectedAuthors,
            Description = string.IsNullOrWhiteSpace(Description) ? null : Description
        };
    }

    private DateTimeOffset? ComputeDate()
    {
        if (!TimeSpan.TryParseExact(SelectedTimeZone, @"\+hh\:mm", CultureInfo.InvariantCulture, out var offset)
            && !TimeSpan.TryParseExact(SelectedTimeZone, @"\-hh\:mm", CultureInfo.InvariantCulture, out offset))
        {
            offset = DateTimeOffset.Now.Offset;
        }

        var localDate = SelectedDate.Date.Add(SelectedTime);
        return new DateTimeOffset(localDate, offset);
    }

    private void UpdatePreview()
    {
        try
        {
            var frontMatter = BuildFrontMatter();
            var slug = SlugGenerator.Generate(frontMatter.Title);
            var date = ComputeDate();
            FileNamePreview = date.HasValue
                ? Post.BuildFileName(date.Value, slug)
                : $"{slug.Value}.md";

            var yaml = YamlFrontMatterSerializer.Serialize(frontMatter);
            FrontMatterPreview = $"---{Environment.NewLine}{yaml}{Environment.NewLine}---";
        }
        catch
        {
            FileNamePreview = "填写标题后生成文件名";
            FrontMatterPreview = string.Empty;
        }
    }

    private void OnCurrentProjectChanged(object? sender, EventArgs e)
    {
        _ = LoadAuthorsAsync();
    }

    private static IReadOnlyList<string> BuildTimeZoneOptions()
    {
        var options = new List<string>();
        for (var minutes = -12 * 60; minutes <= 14 * 60; minutes += 30)
        {
            var offset = TimeSpan.FromMinutes(minutes);
            options.Add(FormatOffset(offset));
        }

        return options;
    }

    private static string FormatOffset(TimeSpan offset)
    {
        var sign = offset >= TimeSpan.Zero ? "+" : "-";
        var absolute = offset.Duration();
        return $"{sign}{absolute.Hours:D2}:{absolute.Minutes:D2}";
    }
}

/// <summary>
/// 作者多选项。
/// </summary>
public sealed partial class AuthorOption : ObservableObject
{
    private readonly Action _onSelectionChanged;

    public string Id { get; }

    public string DisplayName { get; }

    [ObservableProperty]
    private bool _isSelected;

    public AuthorOption(Author author, Action onSelectionChanged)
    {
        Id = author.Id;
        DisplayName = $"{author.Id} ({author.Name})";
        _onSelectionChanged = onSelectionChanged;
    }

    partial void OnIsSelectedChanged(bool value)
    {
        _onSelectionChanged();
    }
}
