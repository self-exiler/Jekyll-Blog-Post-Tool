using JekyllPostTool.Application.Ai;
using JekyllPostTool.Application.Authors;
using JekyllPostTool.Application.Posts;
using JekyllPostTool.Application.Projects;
using JekyllPostTool.Domain.Authors;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;
using JekyllPostTool.Infrastructure.Ai;
using JekyllPostTool.Infrastructure.FileSystem;
using JekyllPostTool.Infrastructure.Import;
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

    private MainWindow? _mainWindow;

    // ADR-009: %APPDATA%\JekyllPostTool\（Roaming）
    private static readonly string AppDataDir = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
        "JekyllPostTool");

    public App()
    {
        InitializeComponent();

        UnhandledException += OnUnhandledException;
    }

    /// <summary>
    /// Invoked when the application is launched.
    /// </summary>
    protected override void OnLaunched(Microsoft.UI.Xaml.LaunchActivatedEventArgs args)
    {
        // 先创建窗口（MainPage.Loaded 不会在激活前触发）
        _mainWindow = new MainWindow();

        // 注册所有服务（含窗口实例）
        var services = new ServiceCollection();
        ConfigureServices(services);
        services.AddSingleton(_mainWindow);
        services.AddSingleton<IDialogService>(_ => new WinUIDialogService(_mainWindow));
        services.AddSingleton<IFilePickerService>(_ => new WinUIFilePickerService(_mainWindow));
        Services = services.BuildServiceProvider();

        // 激活窗口后 MainPage.Loaded 才触发 → 此时 Services 已就绪
        _mainWindow.Activate();

        _ = InitializeProjectAsync();
    }

    private static void ConfigureServices(IServiceCollection services)
    {
        services.AddSingleton<IPostRepository, FilePostRepository>();
        services.AddSingleton<IProjectContext, ProjectContext>();
        services.AddSingleton<IAuthorRepository>(sp => new YamlAuthorRepository(
            () => sp.GetRequiredService<IProjectContext>().CurrentProject?.AuthorsFilePath));

        services.AddSingleton(new DefaultProjectSettingService(AppDataDir));
        services.AddSingleton(new AiSettingsService(AppDataDir));

        // AI 服务：共享 HttpClient 实例，配置在每次调用时读取（支持运行时修改）
        services.AddSingleton<HttpClient>();
        services.AddSingleton<IAiService>(sp => new OpenAiService(
            sp.GetRequiredService<HttpClient>(),
            sp.GetRequiredService<AiSettingsService>()));

        services.AddTransient<FilenameConflictResolver>();
        services.AddTransient<PostCreateUseCase>();
        services.AddTransient<PostEditUseCase>();
        services.AddTransient<AuthorCrudUseCase>();
        services.AddTransient<MarkdownBodyImporter>();
        services.AddTransient<ImageInserter>();

        services.AddTransient<ProjectPageViewModel>();
        services.AddTransient<AuthorsPageViewModel>();
        // Singleton：博文头信息页与博文正文页共享同一份编辑状态
        services.AddSingleton<PostPageViewModel>();
        services.AddTransient<AdvancedPageViewModel>();
    }

    private async Task InitializeProjectAsync()
    {
        try
        {
            var projectContext = Services.GetRequiredService<IProjectContext>();
            var settingsService = Services.GetRequiredService<DefaultProjectSettingService>();

            var defaultPath = await settingsService.GetAsync();
            if (!string.IsNullOrWhiteSpace(defaultPath) && Directory.Exists(defaultPath))
            {
                projectContext.CurrentProject = new BlogProject(defaultPath);
            }
        }
        catch
        {
            // 设置加载失败不影响应用启动
        }
    }

    private void OnUnhandledException(object sender, Microsoft.UI.Xaml.UnhandledExceptionEventArgs e)
    {
        // 写入崩溃日志便于事后诊断，但不吞异常（e.Handled = false 让进程正常崩溃，避免在不一致状态下继续运行）
        try
        {
            var logPath = Path.Combine(AppDataDir, "crash.log");
            Directory.CreateDirectory(AppDataDir);
            File.AppendAllText(logPath, $"[{DateTime.Now:O}] {e.Exception}\r\n\r\n");
        }
        catch
        {
            // 日志写入失败时忽略
        }

        e.Handled = false;
    }
}
