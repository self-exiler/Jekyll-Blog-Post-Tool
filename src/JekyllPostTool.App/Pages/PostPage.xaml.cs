using JekyllPostTool_App.ViewModels;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.UI.Xaml.Controls;

namespace JekyllPostTool_App.Pages;

/// <summary>
/// 博文编辑页。
/// </summary>
public sealed partial class PostPage : Page
{
    public PostPageViewModel ViewModel { get; }

    public PostPage()
    {
        InitializeComponent();
        ViewModel = App.Current.Services.GetRequiredService<PostPageViewModel>();
        DataContext = ViewModel;
    }

    private void OnLoaded(object sender, Microsoft.UI.Xaml.RoutedEventArgs e)
    {
        ViewModel.LoadAuthorsCommand.Execute(null);
    }
}
