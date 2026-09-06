using System.ComponentModel;
using JekyllPostTool_App.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace JekyllPostTool_App.Pages;

/// <summary>
/// 博文正文编辑页。
/// </summary>
public sealed partial class PostBodyPage : Page
{
    private const string TableSnippet = "| 列1 | 列2 | 列3 |";

    private readonly DispatcherTimer _previewTimer;

    public PostPageViewModel ViewModel { get; }

    public PostBodyPage()
    {
        InitializeComponent();

        // 与 PostPage 共享同一 Singleton 实例
        ViewModel = App.Current.PostPageViewModel;
        DataContext = ViewModel;

        // 预览防抖：避免每次按键都重建整个富文本树
        _previewTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(300) };
        _previewTimer.Tick += (_, _) =>
        {
            _previewTimer.Stop();
            MarkdownPreview.Text = ViewModel.Body;
        };

        ViewModel.PropertyChanged += OnViewModelPropertyChanged;
        MarkdownPreview.Text = ViewModel.Body;
    }

    private void OnViewModelPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(PostPageViewModel.Body))
        {
            _previewTimer.Stop();
            _previewTimer.Start();
        }
    }

    /// <summary>
    /// 插入图片到正文光标处。
    /// </summary>
    private async void OnInsertImagesClick(object sender, RoutedEventArgs e)
    {
        // 确保 Body 与 TextBox 当前文本同步（避免 x:Bind 延迟导致光标位置与 Body 不匹配）
        ViewModel.Body = BodyTextBox.Text;
        await ViewModel.InsertImagesCommand.ExecuteAsync((int?)BodyTextBox.SelectionStart);
    }

    /// <summary>
    /// 切换实时预览分栏。
    /// </summary>
    private void OnPreviewToggleClick(object sender, RoutedEventArgs e)
    {
        var show = PreviewToggle.IsChecked == true;
        PreviewBorder.Visibility = show ? Visibility.Visible : Visibility.Collapsed;
        PreviewColumn.Width = show ? new GridLength(1, GridUnitType.Star) : new GridLength(0);
    }

    // --- 工具栏命令 ---

    private void OnBoldClick(object sender, RoutedEventArgs e) => ApplyWrap("**");

    private void OnItalicClick(object sender, RoutedEventArgs e) => ApplyWrap("*");

    private void OnStrikethroughClick(object sender, RoutedEventArgs e) => ApplyWrap("~~");

    private void OnInlineCodeClick(object sender, RoutedEventArgs e) => ApplyWrap("`");

    private void OnHeading2Click(object sender, RoutedEventArgs e) => ApplyLinePrefix("## ");

    private void OnHeading3Click(object sender, RoutedEventArgs e) => ApplyLinePrefix("### ");

    private void OnQuoteClick(object sender, RoutedEventArgs e) => ApplyLinePrefix("> ");

    private void OnBulletListClick(object sender, RoutedEventArgs e) => ApplyLinePrefix("- ");

    private void OnOrderedListClick(object sender, RoutedEventArgs e) => ApplyLinePrefix("1. ");

    private void OnCodeBlockClick(object sender, RoutedEventArgs e) => ApplyCodeBlock();

    private void OnLinkClick(object sender, RoutedEventArgs e) => ApplyLink();

    private void OnTableClick(object sender, RoutedEventArgs e) => InsertSnippet(
        string.Join(Environment.NewLine, TableSnippet, "| --- | --- | --- |", "|  |  |  |"));

    // --- 编辑辅助 ---

    /// <summary>
    /// 用标记包裹当前选区（选区两侧已有相同标记时解除包裹；无选区时插入成对标记并让光标居中）。
    /// </summary>
    private void ApplyWrap(string marker)
    {
        var text = BodyTextBox.Text;
        var start = BodyTextBox.SelectionStart;
        var length = BodyTextBox.SelectionLength;

        if (start >= marker.Length
            && start + length + marker.Length <= text.Length
            && text.Substring(start - marker.Length, marker.Length) == marker
            && text.Substring(start + length, marker.Length) == marker)
        {
            // 解除包裹
            text = text.Remove(start + length, marker.Length).Remove(start - marker.Length, marker.Length);
            BodyTextBox.Text = text;
            BodyTextBox.SelectionStart = start - marker.Length;
            BodyTextBox.SelectionLength = length;
        }
        else if (length > 0)
        {
            text = text.Insert(start + length, marker).Insert(start, marker);
            BodyTextBox.Text = text;
            BodyTextBox.SelectionStart = start + marker.Length;
            BodyTextBox.SelectionLength = length;
        }
        else
        {
            text = text.Insert(start, marker + marker);
            BodyTextBox.Text = text;
            BodyTextBox.SelectionStart = start + marker.Length;
        }

        SyncBodyFromTextBox();
    }

    /// <summary>
    /// 给光标/选区覆盖的每一行加行首前缀（首行已有前缀时改为移除）。
    /// </summary>
    private void ApplyLinePrefix(string prefix)
    {
        var text = BodyTextBox.Text;
        var start = BodyTextBox.SelectionStart;
        var selEnd = start + BodyTextBox.SelectionLength;

        var lineStarts = CollectLineStarts(text, start, selEnd);
        if (lineStarts.Count == 0)
        {
            return;
        }

        var removing = text.Substring(lineStarts[0]).StartsWith(prefix, StringComparison.Ordinal);

        // 从最后一行往前改，前面的偏移不受影响
        var startDelta = 0;
        var endDelta = 0;
        for (var i = lineStarts.Count - 1; i >= 0; i--)
        {
            var at = lineStarts[i];
            if (removing && !text.Substring(at).StartsWith(prefix, StringComparison.Ordinal))
            {
                continue;
            }

            text = removing ? text.Remove(at, prefix.Length) : text.Insert(at, prefix);

            // 行首在选区起止位置之前的编辑才影响对应位置
            if (at <= start)
            {
                startDelta += removing ? -prefix.Length : prefix.Length;
            }
            if (at <= selEnd)
            {
                endDelta += removing ? -prefix.Length : prefix.Length;
            }
        }

        BodyTextBox.Text = text;
        var newStart = Math.Max(0, start + startDelta);
        BodyTextBox.SelectionStart = newStart;
        BodyTextBox.SelectionLength = Math.Max(0, selEnd + endDelta - newStart);
        SyncBodyFromTextBox();
    }

    /// <summary>
    /// 用 ``` 围栏包裹选区（或插入空代码块），光标移到围栏内容首。
    /// </summary>
    private void ApplyCodeBlock()
    {
        const string fence = "```";
        var start = BodyTextBox.SelectionStart;
        var length = BodyTextBox.SelectionLength;
        var selected = BodyTextBox.Text.Substring(start, length).TrimEnd('\r', '\n');

        var inner = selected.Length == 0 ? string.Empty : selected + Environment.NewLine;
        var snippet = fence + Environment.NewLine + inner + fence;

        BodyTextBox.Text = BodyTextBox.Text.Remove(start, length).Insert(start, snippet);
        BodyTextBox.SelectionStart = start + fence.Length + Environment.NewLine.Length;
        SyncBodyFromTextBox();
    }

    /// <summary>
    /// 插入链接：选中 URL 时把选中内容放进括号，否则把选中内容/占位符放进方括号，并选中待填部分。
    /// </summary>
    private void ApplyLink()
    {
        var start = BodyTextBox.SelectionStart;
        var length = BodyTextBox.SelectionLength;
        var selected = BodyTextBox.Text.Substring(start, length);

        string insert;
        int selStart, selLength;
        if (length > 0
            && Uri.TryCreate(selected, UriKind.Absolute, out var uri)
            && uri.Scheme is "http" or "https")
        {
            insert = $"[链接文本]({selected})";
            selStart = start + 1;
            selLength = 4;
        }
        else
        {
            var label = length > 0 ? selected : "链接文本";
            insert = $"[{label}](url)";
            selStart = start + label.Length + 3;
            selLength = 3;
        }

        BodyTextBox.Text = BodyTextBox.Text.Remove(start, length).Insert(start, insert);
        BodyTextBox.SelectionStart = selStart;
        BodyTextBox.SelectionLength = selLength;
        SyncBodyFromTextBox();
    }

    /// <summary>
    /// 在光标处插入片段（不在行首时先补换行，光标移到片段之后）。
    /// </summary>
    private void InsertSnippet(string snippet)
    {
        var text = BodyTextBox.Text;
        var start = BodyTextBox.SelectionStart;
        var length = BodyTextBox.SelectionLength;

        var atLineStart = start == 0 || text[start - 1] == '\n';
        var insert = (atLineStart ? string.Empty : Environment.NewLine) + snippet + Environment.NewLine;

        BodyTextBox.Text = text.Remove(start, length).Insert(start, insert);
        BodyTextBox.SelectionStart = start + insert.Length;
        SyncBodyFromTextBox();
    }

    /// <summary>
    /// 收集光标/选区覆盖到的行首（选区以换行结尾时不包含下一行）。
    /// </summary>
    private static List<int> CollectLineStarts(string text, int start, int selEnd)
    {
        var lineStarts = new List<int>();
        var lineStart = start == 0 ? 0 : text.LastIndexOf('\n', start - 1) + 1;
        while (lineStart <= selEnd)
        {
            if (lineStart < selEnd || start == selEnd)
            {
                lineStarts.Add(lineStart);
            }

            var newline = text.IndexOf('\n', lineStart);
            if (newline < 0 || newline >= selEnd)
            {
                break;
            }

            lineStart = newline + 1;
        }

        return lineStarts;
    }

    /// <summary>
    /// 把 TextBox 当前文本同步回 ViewModel（触发预览防抖刷新）。
    /// </summary>
    private void SyncBodyFromTextBox()
    {
        ViewModel.Body = BodyTextBox.Text;
        BodyTextBox.Focus(FocusState.Programmatic);
    }
}
