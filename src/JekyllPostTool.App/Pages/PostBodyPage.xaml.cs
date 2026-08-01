using JekyllPostTool_App.ViewModels;
using Microsoft.Extensions.DependencyInjection;
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
        ViewModel = App.Current.Services.GetRequiredService<PostPageViewModel>();
        DataContext = ViewModel;
    }

    /// <summary>
    /// 插入图片到正文光标处。
    /// </summary>
    private async void OnInsertImagesClick(object sender, RoutedEventArgs e)
    {
        await ViewModel.InsertImagesCommand.ExecuteAsync((int?)BodyTextBox.SelectionStart);
    }
}
