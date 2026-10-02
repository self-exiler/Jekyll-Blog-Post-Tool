//! 应用服务聚合：手写组合根，对偶 .NET 版 `App.xaml.cs`——全部单例、无生命周期差异。
//!
//! 图片插入与 `_posts` 读写都是 `jp_infrastructure` 的纯函数 / 无状态仓储，
//! 由动作层直接调用，不进服务表。

pub mod dialogs;
pub mod pickers;
pub mod project_context;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use windows::Win32::Foundation::HWND;
use windows_core::Result as WinResult;
use winui3::Microsoft::UI::Windowing::AppWindow;
use winui3::Microsoft::UI::Xaml::{UIElement, Window, XamlRoot};

use jp_application::ai::AiSettingsService;
use jp_application::authors::AuthorCrudUseCase;
use jp_application::posts::{FilenameConflictResolver, PostSaveUseCase};
use jp_application::projects::DefaultProjectSettingService;
use jp_domain::authors::AuthorRepository;
use jp_domain::posts::PostRepository;
use jp_infrastructure::ai::{KeywordExtractor, OpenAiClient};
use jp_infrastructure::filesystem::{FilePostRepository, YamlAuthorRepository};

use crate::view_models::PostForm;
pub use project_context::ProjectContext;

/// 跨线程共享的服务集合（全部字段 `Send + Sync`）。
pub struct Services {
    pub project: ProjectContext,
    pub settings: DefaultProjectSettingService,
    pub ai_settings: AiSettingsService,
    pub author_repository: Arc<dyn AuthorRepository>,
    pub save_use_case: Arc<PostSaveUseCase>,
    pub author_use_case: Arc<AuthorCrudUseCase>,
    pub ai: Arc<dyn KeywordExtractor>,
    /// 博文头信息页与博文正文页共享同一份编辑状态（对应 `App.Current.PostPageViewModel`）。
    pub post_form: Arc<Mutex<PostForm>>,
}

static SERVICES: OnceLock<Arc<Services>> = OnceLock::new();
static WINDOW: OnceLock<Window> = OnceLock::new();

impl Services {
    /// 编辑态快照：动作层先取快照再跑阻塞流程——锁绝不能横跨对话框（那会阻塞 UI 线程）。
    pub fn form(&self) -> PostForm {
        self.lock_form().clone()
    }

    /// 原地修改编辑态（UI 线程回填、工作线程落状态都用它）。
    pub fn edit_form<T>(&self, change: impl FnOnce(&mut PostForm) -> T) -> T {
        change(&mut self.lock_form())
    }

    fn lock_form(&self) -> MutexGuard<'_, PostForm> {
        self.post_form
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// 装配全部服务；`app_data` 为 `%APPDATA%\JekyllPostTool`。
pub fn compose_services(app_data: &Path) -> Arc<Services> {
    let project = ProjectContext::default();

    let post_repository: Arc<dyn PostRepository> = Arc::new(FilePostRepository::new());
    let author_repository: Arc<dyn AuthorRepository> = Arc::new(YamlAuthorRepository::new(
        project.authors_path_resolver(),
    ));

    let services = Arc::new(Services {
        project: project.clone(),
        settings: DefaultProjectSettingService::new(app_data),
        ai_settings: AiSettingsService::new(app_data),
        author_repository: Arc::clone(&author_repository),
        save_use_case: Arc::new(PostSaveUseCase::new(
            Arc::clone(&post_repository),
            Arc::clone(&author_repository),
            FilenameConflictResolver::new(post_repository),
        )),
        author_use_case: Arc::new(AuthorCrudUseCase::new(author_repository)),
        ai: OpenAiClient::new(AiSettingsService::new(app_data)).into_shared(),
        post_form: Arc::new(Mutex::new(PostForm::empty(PostForm::now()))),
    });

    let _ = SERVICES.set(Arc::clone(&services));
    services
}

pub fn services() -> Arc<Services> {
    Arc::clone(
        SERVICES
            .get()
            .expect("服务尚未装配：compose_services 必须在 Application::Start 之后调用"),
    )
}

/// 窗口就绪后登记，供对话框与文件选择器取 `XamlRoot` / HWND。
pub fn attach_window(window: &Window) {
    let _ = WINDOW.set(window.clone());
}

pub fn window() -> Option<Window> {
    WINDOW.get().cloned()
}

pub fn app_window() -> Option<AppWindow> {
    window().and_then(|window| window.AppWindow().ok())
}

/// 顶层窗口句柄。
pub fn hwnd() -> Option<HWND> {
    app_window().and_then(|app_window| hwnd_of(&app_window).ok())
}

/// `AppWindow.Id()` 只是个值结构体 `WindowId`，取 HWND 必须走 FrameworkUdk 的 interop 入口。
pub fn hwnd_of(app_window: &AppWindow) -> WinResult<HWND> {
    let window_id = app_window.Id()?;
    // SAFETY: interop 由 Microsoft.Internal.FrameworkUdk.dll 提供，进程内 WinUI 运行时已加载
    unsafe { winui3::interop::GetWindowFromWindowId(window_id) }
}

/// 对话框必须挂到 XamlRoot，否则 WinUI 抛「No XamlRoot」。
pub fn xaml_root() -> WinResult<XamlRoot> {
    window()
        .ok_or_else(windows_core::Error::empty)
        .and_then(|window| window.Content())
        .and_then(|content: UIElement| content.XamlRoot())
}
