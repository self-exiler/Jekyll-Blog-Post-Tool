using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using JekyllPostTool.Application.Ai;
using JekyllPostTool.Application.Authors;
using JekyllPostTool.Application.Posts;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Infrastructure.Import;
using JekyllPostTool_App.Services;
using Microsoft.UI.Xaml;

namespace JekyllPostTool_App.ViewModels;

/// <summary>
/// 博文编辑页视图模型（核心 partial：字段、构造、属性、作者、预览）。
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
    private readonly IAiService _aiService;
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
        IProjectContext projectContext,
        PostCreateUseCase postCreateUseCase,
        PostEditUseCase postEditUseCase,
        AuthorCrudUseCase authorUseCase,
        IFilePickerService filePickerService,
        IDialogService dialogService,
        MarkdownBodyImporter bodyImporter,
        IAiService aiService,
        ImageInserter imageInserter)
    {
        _projectContext = projectContext;
        _postCreateUseCase = postCreateUseCase;
        _postEditUseCase = postEditUseCase;
        _authorUseCase = authorUseCase;
        _filePickerService = filePickerService;
        _dialogService = dialogService;
        _bodyImporter = bodyImporter;
        _aiService = aiService;
        _imageInserter = imageInserter;

        _previewTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(150) };
        _previewTimer.Tick += OnPreviewTimerTick;

        _projectContext.CurrentProjectChanged += OnCurrentProjectChanged;
    }

    /// <summary>
    /// 获取当前博文 slug：优先从已保存文件名提取，否则用标题生成（遵循 ADR-006 中文保留）。
    /// </summary>
    private string GetCurrentSlug()
    {
        if (!string.IsNullOrEmpty(_originalFilePath))
        {
            return ImageInserter.ExtractSlugFromFileName(_originalFilePath);
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

        var authors = await _authorUseCase.ListAsync();
        foreach (var author in authors)
        {
            AvailableAuthors.Add(new AuthorOption(author, OnAuthorSelectionChanged));
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

    /// <summary>
    /// 由编辑状态构造 FrontMatter（委托 FrontMatterBuilder 纯函数）。
    /// </summary>
    private FrontMatter BuildFrontMatter() => FrontMatterBuilder.Build(BuildEditState());

    private PostEditState BuildEditState()
    {
        var selectedAuthorIds = AvailableAuthors
            .Where(a => a.IsSelected)
            .Select(a => a.Id)
            .ToList();

        return new PostEditState(
            Title,
            Category1,
            Category2,
            Tags,
            Description,
            SelectedDate,
            SelectedTime,
            SelectedTimeZone,
            selectedAuthorIds);
    }

    private void UpdatePreview()
    {
        var frontMatter = FrontMatterBuilder.Build(BuildEditState());
        var (fileNamePreview, frontMatterPreview) = FrontMatterBuilder.BuildPreview(frontMatter);
        FileNamePreview = fileNamePreview;
        FrontMatterPreview = frontMatterPreview;
    }

    private void NotifyPostFileChanged()
    {
        OnPropertyChanged(nameof(TargetDirectory));
        InsertImagesCommand.NotifyCanExecuteChanged();
    }

    private void OnCurrentProjectChanged(object? sender, EventArgs e)
    {
        _ = LoadAuthorsAsync();
        NotifyPostFileChanged();
    }
}
