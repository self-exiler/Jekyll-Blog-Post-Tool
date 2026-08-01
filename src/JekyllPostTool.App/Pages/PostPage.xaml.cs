using JekyllPostTool_App.ViewModels;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace JekyllPostTool_App.Pages;

/// <summary>
/// 博文编辑页。
/// </summary>
public sealed partial class PostPage : Page
{
    /// <summary>
    /// 页面内容区宽度（逻辑像素）达到该值时采用左右分栏，否则上下堆叠。
    /// </summary>
    private const double WideLayoutMinWidth = 820;

    public PostPageViewModel ViewModel { get; }

    public PostPage()
    {
        InitializeComponent();
        ViewModel = App.Current.Services.GetRequiredService<PostPageViewModel>();
        DataContext = ViewModel;
    }

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        ViewModel.LoadAuthorsCommand.Execute(null);
    }

    private void OnSizeChanged(object sender, SizeChangedEventArgs e)
    {
        var state = e.NewSize.Width >= WideLayoutMinWidth ? "WideLayout" : "NarrowLayout";
        VisualStateManager.GoToState(this, state, false);
    }
}
