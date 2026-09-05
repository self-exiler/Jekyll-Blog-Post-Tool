using JekyllPostTool_App.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace JekyllPostTool_App.Pages;

/// <summary>
/// 项目设置页。
/// </summary>
public sealed partial class ProjectPage : Page
{
    public ProjectPageViewModel ViewModel { get; }

    public ProjectPage()
    {
        InitializeComponent();
        ViewModel = App.Current.CreateProjectPageViewModel();
        DataContext = ViewModel;
    }

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        // 页面有 NavigationCacheMode，每次进入都重新同步显示状态
        ViewModel.RefreshFromContext();
    }
}
