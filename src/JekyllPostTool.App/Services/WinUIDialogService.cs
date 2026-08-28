using JekyllPostTool.Application.Posts;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace JekyllPostTool_App.Services;

/// <summary>
/// 基于 WinUI ContentDialog 的对话框服务。
/// </summary>
public sealed class WinUIDialogService(Window window)
{
    private XamlRoot XamlRoot
    {
        get
        {
            var root = ((FrameworkElement)window.Content).XamlRoot;
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

    /// <summary>
    /// 显示文件名冲突处理选项，返回用户选择的处理方式；取消返回 null。
    /// </summary>
    public async Task<ConflictResolutionKind?> ShowConflictResolutionAsync(string fileName, int? autoSuffix)
    {
        var radioButtons = new RadioButtons();

        radioButtons.Items.Add(new RadioButton
        {
            Content = autoSuffix.HasValue ? $"自动加序号后缀 ({autoSuffix})" : "自动加序号后缀",
            Tag = ConflictResolutionKind.AutoSuffix
        });
        radioButtons.Items.Add(new RadioButton
        {
            Content = "覆盖现有文件",
            Tag = ConflictResolutionKind.Overwrite
        });
        radioButtons.SelectedIndex = 0;

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
