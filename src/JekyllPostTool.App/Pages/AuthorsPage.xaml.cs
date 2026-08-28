using JekyllPostTool_App.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace JekyllPostTool_App.Pages;

/// <summary>
/// 作者管理页。
/// </summary>
public sealed partial class AuthorsPage : Page
{
    public AuthorsPageViewModel ViewModel { get; }

    public AuthorsPage()
    {
        InitializeComponent();
        ViewModel = App.Current.CreateAuthorsPageViewModel();
        DataContext = ViewModel;
    }

    private void OnLoaded(object sender, Microsoft.UI.Xaml.RoutedEventArgs e)
    {
        ViewModel.LoadAuthorsCommand.Execute(null);
    }

    private void OnUnloaded(object sender, Microsoft.UI.Xaml.RoutedEventArgs e)
    {
        ViewModel.Dispose();
    }
}
