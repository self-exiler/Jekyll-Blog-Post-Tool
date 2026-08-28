using System.Collections.ObjectModel;
using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using JekyllPostTool.Application.Authors;
using JekyllPostTool.Domain.Authors;
using JekyllPostTool_App.Services;

namespace JekyllPostTool_App.ViewModels;

/// <summary>
/// 作者管理页视图模型。
/// </summary>
public sealed partial class AuthorsPageViewModel : ObservableObject, IDisposable
{
    private readonly AuthorCrudUseCase _authorUseCase;
    private readonly IAuthorRepository _authorRepository;
    private readonly ProjectContext _projectContext;
    private readonly WinUIDialogService _dialogService;

    [ObservableProperty]
    private ObservableCollection<Author> _authors = new();

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasSelection))]
    [NotifyPropertyChangedFor(nameof(IsReadOnly))]
    private Author? _selectedAuthor;

    [ObservableProperty]
    private string _id = string.Empty;

    [ObservableProperty]
    private string _name = string.Empty;

    [ObservableProperty]
    private string _twitter = string.Empty;

    [ObservableProperty]
    private string _url = string.Empty;

    [ObservableProperty]
    private bool _isNewAuthor;

    public bool HasSelection => SelectedAuthor is not null || IsNewAuthor;

    public bool IsReadOnly => !IsNewAuthor && SelectedAuthor is not null;

    public AuthorsPageViewModel(
        AuthorCrudUseCase authorUseCase,
        IAuthorRepository authorRepository,
        ProjectContext projectContext,
        WinUIDialogService dialogService)
    {
        _authorUseCase = authorUseCase;
        _authorRepository = authorRepository;
        _projectContext = projectContext;
        _dialogService = dialogService;

        // 唯一通知机制：PropertyChanged(nameof(CurrentProject))
        _projectContext.PropertyChanged += OnCurrentProjectChanged;
    }

    partial void OnSelectedAuthorChanged(Author? value)
    {
        if (value is not null)
        {
            Id = value.Id;
            Name = value.Name;
            Twitter = value.Twitter ?? string.Empty;
            Url = value.Url ?? string.Empty;
            IsNewAuthor = false;
        }
    }

    [RelayCommand]
    private async Task LoadAuthorsAsync()
    {
        Authors.Clear();

        if (_projectContext.CurrentProject is null)
        {
            return;
        }

        try
        {
            var authors = await _authorRepository.GetAllAsync();
            foreach (var author in authors)
            {
                Authors.Add(author);
            }
        }
        catch (Exception ex)
        {
            // authors.yml 损坏或被占用时不让异常静默（作者页不再永远空白无提示）
            await _dialogService.ShowInfoAsync("加载作者失败", ex.Message);
        }

        ClearSelection();
    }

    [RelayCommand]
    private void AddAuthor()
    {
        SelectedAuthor = null;
        Id = string.Empty;
        Name = string.Empty;
        Twitter = string.Empty;
        Url = string.Empty;
        IsNewAuthor = true;
    }

    [RelayCommand]
    private async Task SaveAuthorAsync()
    {
        if (_projectContext.CurrentProject is null)
        {
            await _dialogService.ShowInfoAsync("未选择项目", "请先选择一个博客项目。");
            return;
        }

        Author author;
        try
        {
            author = new Author(Id.Trim(), Name.Trim(), Twitter, Url);
        }
        catch (ArgumentException ex)
        {
            await _dialogService.ShowInfoAsync("输入无效", ex.Message);
            return;
        }

        try
        {
            if (IsNewAuthor)
            {
                await _authorUseCase.AddAsync(author);
            }
            else if (SelectedAuthor is not null)
            {
                await _authorUseCase.UpdateAsync(author);
            }
            else
            {
                return;
            }
        }
        catch (InvalidOperationException ex)
        {
            await _dialogService.ShowInfoAsync("保存失败", ex.Message);
            return;
        }

        await LoadAuthorsAsync();
    }

    [RelayCommand]
    private async Task DeleteAuthorAsync()
    {
        if (SelectedAuthor is null)
        {
            return;
        }

        var confirmed = await _dialogService.ShowConfirmAsync(
            "删除作者",
            $"确定要删除作者 {SelectedAuthor.Id} 吗？此操作不可撤销。",
            "删除");

        if (!confirmed)
        {
            return;
        }

        try
        {
            await _authorUseCase.DeleteAsync(SelectedAuthor.Id);
        }
        catch (InvalidOperationException ex)
        {
            await _dialogService.ShowInfoAsync("删除失败", ex.Message);
            return;
        }

        await LoadAuthorsAsync();
    }

    [RelayCommand]
    private void Cancel()
    {
        ClearSelection();
    }

    private void ClearSelection()
    {
        SelectedAuthor = null;
        Id = string.Empty;
        Name = string.Empty;
        Twitter = string.Empty;
        Url = string.Empty;
        IsNewAuthor = false;
    }

    private void OnCurrentProjectChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName != nameof(ProjectContext.CurrentProject))
        {
            return;
        }

        _ = LoadAuthorsAsync().ContinueWith(
            static t => System.Diagnostics.Debug.WriteLine($"[AuthorsPageVM] 加载作者失败: {t.Exception}"),
            TaskContinuationOptions.OnlyOnFaulted);
    }

    public void Dispose()
    {
        _projectContext.PropertyChanged -= OnCurrentProjectChanged;
    }
}
