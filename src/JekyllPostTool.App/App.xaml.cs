using JekyllPostTool.Application.Authors;
using JekyllPostTool.Application.Posts;
using JekyllPostTool.Application.Projects;
using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;
using JekyllPostTool.Domain.Settings;
using JekyllPostTool.Infrastructure.FileSystem;
using JekyllPostTool.Infrastructure.Import;
using JekyllPostTool.Infrastructure.Persistence;
using JekyllPostTool_App.Services;
using JekyllPostTool_App.ViewModels;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.UI.Xaml;

namespace JekyllPostTool_App;

/// <summary>
/// Provides application-specific behavior to supplement the default Application class.
/// </summary>
public partial class App : Application
{
    public static new App Current => (App)Application.Current;

    public IServiceProvider Services { get; private set; } = null!;

    private Window? _window;

    public App()
    {
        InitializeComponent();
    }

    /// <summary>
    /// Invoked when the application is launched.
    /// </summary>
    /// <param name="args">Details about the launch request and process.</param>
    protected override async void OnLaunched(Microsoft.UI.Xaml.LaunchActivatedEventArgs args)
    {
        _window = new MainWindow();

        var serviceCollection = new ServiceCollection();
        ConfigureServices(serviceCollection);
        serviceCollection.AddSingleton(_window);
        serviceCollection.AddSingleton<IDialogService>(sp => new WinUIDialogService(sp.GetRequiredService<MainWindow>()));
        serviceCollection.AddSingleton<IFilePickerService>(sp => new WinUIFilePickerService(sp.GetRequiredService<MainWindow>()));

        Services = serviceCollection.BuildServiceProvider();

        await InitializeProjectAsync();

        _window.Activate();
    }

    private static void ConfigureServices(IServiceCollection services)
    {
        services.AddSingleton<IPostRepository, FilePostRepository>();
        services.AddSingleton<IAppSettingsRepository>(sp => new JsonAppSettingsRepository(
            Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "JekyllPostTool")));
        services.AddSingleton<IProjectContext, ProjectContext>();
        services.AddSingleton<IAuthorRepository, CurrentProjectAuthorRepository>();

        services.AddSingleton<DefaultProjectSettingService>();

        services.AddTransient<FrontMatterValidator>();
        services.AddTransient<FilenameConflictResolver>();
        services.AddTransient<PostCreateUseCase>();
        services.AddTransient<PostEditUseCase>();
        services.AddTransient<AuthorCrudUseCase>();
        services.AddTransient<MarkdownBodyImporter>();

        services.AddTransient<ProjectPageViewModel>();
        services.AddTransient<AuthorsPageViewModel>();
        services.AddTransient<PostPageViewModel>();
    }

    private async Task InitializeProjectAsync()
    {
        var projectContext = Services.GetRequiredService<IProjectContext>();
        var settingsService = Services.GetRequiredService<DefaultProjectSettingService>();

        var defaultPath = await settingsService.GetAsync();
        if (!string.IsNullOrWhiteSpace(defaultPath) && Directory.Exists(defaultPath))
        {
            projectContext.CurrentProject = new BlogProject(defaultPath);
        }
    }
}
