using CommunityToolkit.Mvvm.ComponentModel;
using JekyllPostTool.Domain.Authors;

namespace JekyllPostTool_App.ViewModels;

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
