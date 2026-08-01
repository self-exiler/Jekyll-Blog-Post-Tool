using JekyllPostTool_App.Pages;
using Microsoft.UI.Xaml.Controls;

namespace JekyllPostTool_App;

/// <summary>
/// 左侧导航外壳，负责四大模块切换。
/// </summary>
public sealed partial class MainPage : Page
{
    public MainPage()
    {
        InitializeComponent();
        Loaded += OnLoaded;
    }

    private void OnLoaded(object sender, Microsoft.UI.Xaml.RoutedEventArgs e)
    {
        NavView.SelectedItem = NavView.MenuItems[0];
        ContentFrame.Navigate(typeof(ProjectPage));
    }

    private void NavView_SelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
    {
        var tag = (string)((NavigationViewItem)args.SelectedItem).Tag;
        var pageType = tag switch
        {
            "Project" => typeof(ProjectPage),
            "Authors" => typeof(AuthorsPage),
            "PostHeader" => typeof(PostPage),
            "PostBody" => typeof(PostBodyPage),
            "Advanced" => typeof(AdvancedPage),
            _ => typeof(ProjectPage)
        };

        if (ContentFrame.CurrentSourcePageType != pageType)
        {
            ContentFrame.Navigate(pageType);
        }
    }
}
