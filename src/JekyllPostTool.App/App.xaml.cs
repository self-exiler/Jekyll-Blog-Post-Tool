using JekyllPostTool.Application.Ai;
using JekyllPostTool.Application.Authors;
using JekyllPostTool.Application.Posts;
using JekyllPostTool.Application.Projects;
using JekyllPostTool.Domain.Posts;
using JekyllPostTool.Domain.Projects;
using JekyllPostTool.Infrastructure.Ai;
using JekyllPostTool.Infrastructure.FileSystem;
using JekyllPostTool_App.Services;
using JekyllPostTool_App.ViewModels;
using Microsoft.UI.Xaml;

namespace JekyllPostTool_App;

/// <summary>
/// Provides application-specific behavior to supplement the default Application class.
/// </summary>
public partial class App : Application
{
    public static new App Current => (App)Application.Current;

    private MainWindow? _mainWindow;

    // ADR-009: %APPDATA%\JekyllPostTool\（Roaming）
    private static readonly string AppDataDir = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
        "JekyllPostTool");

    // 手写组合根：全部单例、无生命周期差异，容器注册仪式无收益
    public ProjectContext ProjectContext { get; } = new();
    public DefaultProjectSettingService SettingService { get; } = new(AppDataDir);
    public AiSettingsService AiSettingsService { get; } = new(AppDataDir);
    public IPostRepository PostRepository { get; } = new FilePostRepository();
    public ImageInserter ImageInserter { get; } = new();

    // 依赖窗口实例，在 OnLaunched 装配
    public WinUIDialogService DialogService { get; private set; } = null!;
    public WinUIFilePickerService FilePickerService { get; private set; } = null!;
    public YamlAuthorRepository AuthorRepository { get; private set; } = null!;
    public OpenAiService AiService { get; private set; } = null!;

    /// <summary>博文头信息页与博文正文页共享同一份编辑状态。</summary>
    public PostPageViewModel PostPageViewModel { get; private set; } = null!;

    public AuthorCrudUseCase AuthorCrudUseCase { get; private set; } = null!;

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

        DialogService = new WinUIDialogService(_mainWindow);
        FilePickerService = new WinUIFilePickerService(_mainWindow);

        // 作者路径经惰性解析器读取，支持运行时切换项目
        AuthorRepository = new YamlAuthorRepository(() => ProjectContext.CurrentProject?.AuthorsFilePath);

        // AI 服务：共享 HttpClient 实例，配置在每次调用时读取（支持运行时修改）
        AiService = new OpenAiService(new HttpClient(), AiSettingsService);

        var conflictResolver = new FilenameConflictResolver(PostRepository);
        AuthorCrudUseCase = new AuthorCrudUseCase(AuthorRepository);

        PostPageViewModel = new PostPageViewModel(
            ProjectContext,
            new PostSaveUseCase(PostRepository, AuthorRepository, conflictResolver),
            AuthorRepository,
            FilePickerService,
            DialogService,
            AiService,
            ImageInserter);

        // 激活窗口后 MainPage.Loaded 才触发 → 此时依赖已就绪
        _mainWindow.Activate();

        _ = InitializeProjectAsync();
    }

    public ProjectPageViewModel CreateProjectPageViewModel() =>
        new(ProjectContext, SettingService, FilePickerService, DialogService);

    public AuthorsPageViewModel CreateAuthorsPageViewModel() =>
        new(AuthorCrudUseCase, AuthorRepository, ProjectContext, DialogService);

    public AdvancedPageViewModel CreateAdvancedPageViewModel() =>
        new(AiSettingsService, DialogService);

    private async Task InitializeProjectAsync()
    {
        try
        {
            var defaultPath = await SettingService.GetAsync();
            if (!string.IsNullOrWhiteSpace(defaultPath) && Directory.Exists(defaultPath))
            {
                ProjectContext.CurrentProject = new BlogProject(defaultPath);
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
