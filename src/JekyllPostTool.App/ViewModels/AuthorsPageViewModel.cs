using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using JekyllPostTool.Application.Authors;
using JekyllPostTool.Domain.Authors;
using JekyllPostTool_App.Services;

namespace JekyllPostTool_App.ViewModels;

/// <summary>
/// 作者管理页视图模型。
/// </summary>
public sealed partial class AuthorsPageViewModel : ObservableObject
{
    private readonly AuthorCrudUseCase _authorUseCase;
    private readonly IProjectContext _projectContext;
    private readonly IDialogService _dialogService;

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
        IProjectContext projectContext,
        IDialogService dialogService)
    {
        _authorUseCase = authorUseCase;
        _projectContext = projectContext;
        _dialogService = dialogService;

        _projectContext.CurrentProjectChanged += OnCurrentProjectChanged;
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

        var authors = await _authorUseCase.ListAsync();
        foreach (var author in authors)
        {
            Authors.Add(author);
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

        AuthorOperationResult result;
        if (IsNewAuthor)
        {
            result = await _authorUseCase.AddAsync(author);
        }
        else if (SelectedAuthor is not null)
        {
            result = await _authorUseCase.UpdateAsync(author);
        }
        else
        {
            return;
        }

        if (!result.IsSuccess)
        {
            await _dialogService.ShowInfoAsync("保存失败", result.Error ?? "未知错误");
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

        var result = await _authorUseCase.DeleteAsync(SelectedAuthor.Id);
        if (!result.IsSuccess)
        {
            await _dialogService.ShowInfoAsync("删除失败", result.Error ?? "未知错误");
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

    private void OnCurrentProjectChanged(object? sender, EventArgs e)
    {
        _ = LoadAuthorsAsync();
    }
}
