using JekyllPostTool.Application.Posts;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace JekyllPostTool_App.Services;

/// <summary>
/// 基于 WinUI ContentDialog 的对话框服务实现。
/// </summary>
public sealed class WinUIDialogService : IDialogService
{
    private readonly Window _window;

    public WinUIDialogService(Window window)
    {
        _window = window;
    }

    private XamlRoot XamlRoot
    {
        get
        {
            var root = ((FrameworkElement)_window.Content).XamlRoot;
            if (root is null)
            {
                throw new InvalidOperationException("XamlRoot 尚未初始化，无法显示对话框。");
            }

            return root;
        }
    }

    public async Task ShowInfoAsync(string title, string message)
    {
        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot,
            Title = title,
            Content = message,
            CloseButtonText = "确定"
        };

        await dialog.ShowAsync();
    }

    public async Task<bool> ShowConfirmAsync(string title, string message, string primaryButtonText = "确认", string closeButtonText = "取消")
    {
        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot,
            Title = title,
            Content = message,
            PrimaryButtonText = primaryButtonText,
            CloseButtonText = closeButtonText
        };

        var result = await dialog.ShowAsync();
        return result == ContentDialogResult.Primary;
    }

    public async Task<ConflictResolutionKind?> ShowConflictResolutionAsync(string fileName, IEnumerable<ConflictResolution> resolutions)
    {
        var radioButtons = new RadioButtons();
        var resolutionList = resolutions.ToList();

        foreach (var resolution in resolutionList)
        {
            var label = resolution.Kind switch
            {
                ConflictResolutionKind.AutoSuffix => $"自动加序号后缀 ({resolution.Suffix})",
                ConflictResolutionKind.Overwrite => "覆盖现有文件",
                _ => resolution.Kind.ToString()
            };

            radioButtons.Items.Add(new RadioButton { Content = label, Tag = resolution.Kind });
        }

        var defaultIndex = resolutionList.FindIndex(r => r.Kind == ConflictResolutionKind.AutoSuffix);
        radioButtons.SelectedIndex = defaultIndex >= 0 ? defaultIndex : 0;

        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot,
            Title = "文件已存在",
            Content = new StackPanel
            {
                Spacing = 12,
                Children =
                {
                    new TextBlock { Text = $"{fileName} 已存在，请选择处理方式：", TextWrapping = TextWrapping.Wrap },
                    radioButtons
                }
            },
            PrimaryButtonText = "确认",
            CloseButtonText = "取消"
        };

        var result = await dialog.ShowAsync();
        if (result != ContentDialogResult.Primary)
        {
            return null;
        }

        return (ConflictResolutionKind?)((RadioButton)radioButtons.SelectedItem).Tag;
    }
}
