using JekyllPostTool_App.ViewModels;
using Microsoft.UI.Xaml.Controls;

namespace JekyllPostTool_App.Pages;

/// <summary>
/// 高级功能页（FR-6.x）。
/// </summary>
public sealed partial class AdvancedPage : Page
{
    public AdvancedPageViewModel ViewModel { get; }

    public AdvancedPage()
    {
        InitializeComponent();
        ViewModel = App.Current.CreateAdvancedPageViewModel();
        DataContext = ViewModel;
    }
}
