using System.Collections.ObjectModel;
using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using JekyllPostTool.Application.Posts;
using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Infrastructure.Ai;
using JekyllPostTool.Infrastructure.FileSystem;
using JekyllPostTool.Infrastructure.Posts;
using JekyllPostTool_App.Services;
using Microsoft.UI.Xaml;

namespace JekyllPostTool_App.ViewModels;

/// <summary>
/// 博文编辑页视图模型（核心 partial：字段、构造、属性、作者、预览）。
/// </summary>
public sealed partial class PostPageViewModel : ObservableObject
{
    private readonly ProjectContext _projectContext;
    private readonly PostSaveUseCase _saveUseCase;
    private readonly IPostRepository _postRepository;
    private readonly IAuthorRepository _authorRepository;
    private readonly WinUIFilePickerService _filePickerService;
    private readonly WinUIDialogService _dialogService;
    private readonly OpenAiService _aiService;
    private readonly ImageInserter _imageInserter;

    private readonly DispatcherTimer _previewTimer;

    private string? _originalFilePath;
    private string? _originalContentHash;

    [ObservableProperty]
    private string _title = string.Empty;

    [ObservableProperty]
    private DateTimeOffset? _selectedDate;

    [ObservableProperty]
    private TimeSpan _selectedTime = DateTimeOffset.Now.TimeOfDay;

    [ObservableProperty]
    private string _selectedTimeZone = TimeZoneFormatter.Format(DateTimeOffset.Now.Offset);

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasCategory1))]
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

    /// <summary>
    /// 导入正文时是否替换而非追加（FR-4.3：默认追加）。
    /// </summary>
    [ObservableProperty]
    private bool _replaceBodyOnImport;

    /// <summary>
    /// 插入图片时统一的 alt 文本（可选，应用于本次所有图片）。
    /// </summary>
    [ObservableProperty]
    private string _altText = string.Empty;

    /// <summary>
    /// 是否正在调用 AI 提取关键字（FR-7.3）。
    /// </summary>
    [ObservableProperty]
    private bool _isExtractingKeywords;

    [ObservableProperty]
    private string _selectedAuthorsDisplay = "无作者";

    public ObservableCollection<AuthorOption> AvailableAuthors { get; } = new();

    /// <summary>
    /// 主分类非空时才允许填写子分类（界面原型设计 §4.3）。
    /// </summary>
    public bool HasCategory1 => !string.IsNullOrWhiteSpace(Category1);

    public IReadOnlyList<string> TimeZoneOptions { get; } = TimeZoneFormatter.BuildOptions();

    /// <summary>
    /// 图片资源目标目录的相对路径（用于展示）。
    /// </summary>
    public string TargetDirectory => _projectContext.CurrentProject is null
        ? "（未选项目）"
        : $"assets/img/{GetCurrentSlug()}/";

    /// <summary>
    /// 是否允许插入图片：已选项目且能确定 slug（有文件名用文件名，否则用 title）。
    /// </summary>
    public bool CanInsertImages => _projectContext.CurrentProject is not null && !string.IsNullOrWhiteSpace(GetCurrentSlug());

    public PostPageViewModel(
        ProjectContext projectContext,
        PostSaveUseCase saveUseCase,
        IPostRepository postRepository,
        IAuthorRepository authorRepository,
        WinUIFilePickerService filePickerService,
        WinUIDialogService dialogService,
        OpenAiService aiService,
        ImageInserter imageInserter)
    {
        _projectContext = projectContext;
        _saveUseCase = saveUseCase;
        _postRepository = postRepository;
        _authorRepository = authorRepository;
        _filePickerService = filePickerService;
        _dialogService = dialogService;
        _aiService = aiService;
        _imageInserter = imageInserter;

        _previewTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(150) };
        _previewTimer.Tick += OnPreviewTimerTick;

        // 唯一通知机制：PropertyChanged(nameof(CurrentProject))（ProjectContext 合一后）
        _projectContext.PropertyChanged += OnCurrentProjectChanged;
    }

    partial void OnIsBusyChanged(bool value) => SavePostCommand.NotifyCanExecuteChanged();

    /// <summary>
    /// 获取当前博文 slug：优先从已保存文件名提取，否则用标题生成（遵循 ADR-006 中文保留）。
    /// </summary>
    private string GetCurrentSlug()
    {
        if (!string.IsNullOrEmpty(_originalFilePath))
        {
            return Post.TryExtractSlug(_originalFilePath);
        }

        // 未保存时用 SlugGenerator 生成，与文件名预览一致（ADR-006：中文原样保留）
        var title = Title;
        if (string.IsNullOrWhiteSpace(title))
        {
            return string.Empty;
        }

        return SlugGenerator.Generate(title).Value;
    }

    private void OnPreviewTimerTick(object? sender, object e)
    {
        _previewTimer.Stop();
        UpdatePreview();
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
            // 防抖：避免每次按键都触发 YAML 序列化与 slug 生成
            _previewTimer.Stop();
            _previewTimer.Start();
        }
    }

    partial void OnTitleChanged(string value)
    {
        // 标题变化影响 slug → 影响插入图片的可用性与目标目录
        OnPropertyChanged(nameof(TargetDirectory));
        OnPropertyChanged(nameof(CanInsertImages));
        InsertImagesCommand.NotifyCanExecuteChanged();
    }

    partial void OnIsExtractingKeywordsChanged(bool value) => ExtractKeywordsCommand.NotifyCanExecuteChanged();

    [RelayCommand]
    public async Task LoadAuthorsAsync()
    {
        AvailableAuthors.Clear();

        if (_projectContext.CurrentProject is null)
        {
            UpdateAuthorsDisplay();
            return;
        }

        try
        {
            var authors = await _authorRepository.GetAllAsync();
            foreach (var author in authors)
            {
                AvailableAuthors.Add(new AuthorOption(author, OnAuthorSelectionChanged));
            }
        }
        catch (Exception ex)
        {
            // authors.yml 损坏或被占用时不让异常静默
            await _dialogService.ShowInfoAsync("加载作者失败", ex.Message);
        }

        UpdateAuthorsDisplay();
        UpdatePreview();
    }

    private void OnAuthorSelectionChanged()
    {
        UpdateAuthorsDisplay();
        UpdatePreview();
    }

    private void UpdateAuthorsDisplay()
    {
        var selected = AvailableAuthors.Where(a => a.IsSelected).Select(a => a.Id).ToList();
        SelectedAuthorsDisplay = selected.Count == 0 ? "无作者" : string.Join(", ", selected);
    }

    /// <summary>由可观察属性收集表单编辑态快照；映射规则全部在 <see cref="PostFormState"/>。</summary>
    private PostFormState BuildFormState() => new(
        Title,
        SelectedDate,
        SelectedTime,
        SelectedTimeZone,
        Category1,
        Category2,
        Tags,
        Description,
        [.. AvailableAuthors.Where(a => a.IsSelected).Select(a => a.Id)]);

    /// <summary>把表单编辑态快照写回可观察属性。</summary>
    private void ApplyFormState(PostFormState state)
    {
        Title = state.Title;
        SelectedDate = state.SelectedDate;
        SelectedTime = state.SelectedTime;
        SelectedTimeZone = state.SelectedTimeZone;
        Category1 = state.Category1;
        Category2 = state.Category2;
        Tags = state.Tags;
        Description = state.Description;
    }

    /// <summary>把表单编辑态写入当前表单并刷新作者选中态。</summary>
    private async Task ApplyFormStateAndRefreshAuthorsAsync(PostFormState state, IReadOnlyList<string> selectedAuthorIds)
    {
        ApplyFormState(state);
        await LoadAuthorsAsync();

        var selectedIds = new HashSet<string>(selectedAuthorIds, StringComparer.Ordinal);
        foreach (var option in AvailableAuthors)
        {
            option.IsSelected = selectedIds.Contains(option.Id);
        }
    }

    private void UpdatePreview()
    {
        try
        {
            var frontMatter = BuildFormState().ToFrontMatter();

            var slug = SlugGenerator.Generate(frontMatter.Title);
            FileNamePreview = frontMatter.Date.HasValue
                ? Post.BuildFileName(frontMatter.Date.Value, slug)
                : $"{slug.Value}.md";

            // 与落盘共用 PostFileFormat.Format：预览即最终字节（所见即所得由构造保证）
            FrontMatterPreview = PostFileFormat.Format(frontMatter);
        }
        catch
        {
            FileNamePreview = "填写标题后生成文件名";
            FrontMatterPreview = string.Empty;
        }
    }

    private void NotifyPostFileChanged()
    {
        OnPropertyChanged(nameof(TargetDirectory));
        OnPropertyChanged(nameof(CanInsertImages));
        InsertImagesCommand.NotifyCanExecuteChanged();
    }

    private void OnCurrentProjectChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName != nameof(ProjectContext.CurrentProject))
        {
            return;
        }

        _ = LoadAuthorsAsync().ContinueWith(
            static t => System.Diagnostics.Debug.WriteLine($"[PostPageVM] 加载作者失败: {t.Exception}"),
            TaskContinuationOptions.OnlyOnFaulted);
        NotifyPostFileChanged();
    }
}
