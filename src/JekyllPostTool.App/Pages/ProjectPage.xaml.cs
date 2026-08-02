using JekyllPostTool_App.ViewModels;
using Microsoft.Extensions.DependencyInjection;
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
        ViewModel = App.Current.Services.GetRequiredService<ProjectPageViewModel>();
        DataContext = ViewModel;
    }

    private void OnUnloaded(object sender, RoutedEventArgs e)
    {
        ViewModel.Dispose();
    }
}
