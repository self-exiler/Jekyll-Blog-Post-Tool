using JekyllPostTool_App.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace JekyllPostTool_App.Pages;

/// <summary>
/// 博文正文编辑页。
/// </summary>
public sealed partial class PostBodyPage : Page
{
    public PostPageViewModel ViewModel { get; }

    public PostBodyPage()
    {
        InitializeComponent();
        // 与 PostPage 共享同一 Singleton 实例
        ViewModel = App.Current.PostPageViewModel;
        DataContext = ViewModel;
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
}
