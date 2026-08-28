using System;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Windows.Graphics;

// To learn more about WinUI, the WinUI project structure,
// and more about our project templates, see: http://aka.ms/winui-project-info.

namespace JekyllPostTool_App;

/// <summary>
/// The application window. This hosts a Frame that displays pages. Add your
/// UI and logic to MainPage.xaml / MainPage.xaml.cs instead of here so you
/// can use Page features such as navigation events and the Loaded lifecycle.
/// </summary>
public sealed partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();

        ExtendsContentIntoTitleBar = true;
        SetTitleBar(AppTitleBar);

        AppWindow.SetIcon("Assets/AppIcon.ico");

        // Navigate the root frame to the main page on startup.
        RootFrame.Navigate(typeof(MainPage));

        // XamlRoot 在内容载入后才可用，届时再做 DPI 换算与窗口尺寸设置
        ((FrameworkElement)Content).Loaded += OnContentLoaded;
    }

    /// <summary>
    /// 界面原型设计 §2.4：默认 1200×800，最小 900×600（逻辑像素）。
    /// AppWindow API 使用物理像素，按 RasterizationScale 换算，
    /// 否则高缩放屏上窗口会比预期小。
    /// </summary>
    private void OnContentLoaded(object sender, RoutedEventArgs e)
    {
        ((FrameworkElement)sender).Loaded -= OnContentLoaded;

        var scale = Content.XamlRoot?.RasterizationScale ?? 1.0;
        if (double.IsNaN(scale) || scale <= 0)
        {
            scale = 1.0;
        }

        AppWindow.Resize(new SizeInt32(
            (int)Math.Round(1200 * scale),
            (int)Math.Round(800 * scale)));

        if (AppWindow.Presenter is OverlappedPresenter presenter)
        {
            presenter.PreferredMinimumWidth = (int)Math.Round(900 * scale);
            presenter.PreferredMinimumHeight = (int)Math.Round(600 * scale);
        }
    }
}
