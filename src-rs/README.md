# src-rs —— Rust / windows-rs 架构

`src/`（.NET 10 + WinUI 3 + CommunityToolkit.Mvvm）的**平行实现**：同一套 Clean Architecture 分层，
换成 Rust + windows-rs，直接对着 WinRT/COM 接口搭 UI。业务语义（校验、文件名、YAML、冲突处理、
外部修改检测）逐条对齐 .NET 侧，不做行为改动。

> **当前状态：已编译、236 个测试全绿、GUI 实机跑通**（2026-09-30，MSVC 工具链，
> WinAppSDK 运行时 2.5.1.0）。五个页面逐一渲染验证，标题输入 → 文件名/front matter
> 预览联动正常。首轮 `cargo check` 修掉约 56 个编译错误（`Param<T>`/`Ref<T>` 形参、
> 控件类的双重 `#[cfg]` 门控、windows crate 缺特性等），运行期修掉两个真 bug，
> 见「已验证的运行期坑」。

## 分层与文件对照

```
src-rs/
├── Cargo.toml                          # workspace：四个 crate，依赖只在 workspace 里声明版本
├── scripts/
│   └── gen-icon-asset.py               #   由 .NET 的 AppIcon.ico 生成 DIB 版窗口图标资源
└── crates/
    ├── jp-domain/                      # ← JekyllPostTool.Domain（零外部依赖，纯逻辑）
    │   ├── posts/                      #   Post / FrontMatter / Slug / MarkdownSplitter / BodyInsertion / ContentHash
    │   ├── authors/                    #   Author + IAuthorRepository（trait）
    │   ├── projects/                   #   BlogProject：_posts 与 authors.yml 路径派生
    │   └── common/                     #   ValidationError、路径工具
    ├── jp-application/                 # ← JekyllPostTool.Application
    │   ├── posts/                      #   PostSaveUseCase / FilenameConflictResolver / validator / form_state / time_zone
    │   ├── authors/                    #   AuthorCrudUseCase
    │   ├── ai/                         #   AiSettingsService + KeywordExtractor trait
    │   ├── projects/                   #   DefaultProjectSettingService（%APPDATA% 下的 json）
    │   └── json_store.rs               #   设置读写的公共骨架
    ├── jp-infrastructure/              # ← JekyllPostTool.Infrastructure
    │   ├── yaml/                       #   parser / emitter / serializer（serde_yaml）
    │   ├── filesystem/                 #   FilePostRepository / YamlAuthorRepository / ImageInserter
    │   ├── ai/openai_client.rs         #   ureq 阻塞 HTTP（原 HttpClient）
    │   └── posts/post_file_format.rs   #   落盘格式
    └── jp-app/                         # ← JekyllPostTool.App（本目录里唯一不可测试的二进制）
        ├── build.rs                    #   构建期把 exe 图标编进 Win32 资源（winresource）
        ├── assets/AppIcon.ico          #   窗口图标（DIB 版，由 scripts/gen-icon-asset.py 转出）
        ├── main.rs                     #   STA + PackageDependency + Application::Start（组合根入口）
        ├── app.rs                      #   OnLaunched + NavigationView + 页面类型解析
        ├── window.rs                   #   标题/尺寸/居中/Mica/深色标题栏（原 MainWindow）
        ├── icon.rs                     #   .ico 解析 + CreateIconIndirect 造 HICON + WM_SETICON
        ├── runtime.rs                  #   DispatcherQueue 派发、工作线程回投、防抖、崩溃日志
        ├── services/                   #   Services 聚合（手写组合根）、对话框、选择器、项目上下文
        ├── actions/                    #   原 RelayCommand：project / post / authors / settings
        ├── view_models/                #   PostForm（两页共享的编辑态）+ AuthorsPage
        ├── pages/                      #   project / authors / post_header / post_body / advanced
        ├── widgets.rs                  #   控件工厂 + 状态推拉
        ├── markdown_edit.rs            #   工具栏对选区的纯文本变换（可单测）
        ├── md_render.rs                #   Markdown → 预览块模型（pulldown-cmark 事件收敛，可单测）
        └── markdown_view.rs            #   块模型 → 预览控件树（近似渲染，替代 MarkdownTextBlock）
```

依赖方向与 .NET 版一致：`jp-app → jp-application → jp-domain`，`jp-infrastructure` 实现
domain/application 里的 trait。单元测试就近放在各 crate 的 `#[cfg(test)]` 里，
共 244 个（domain 42 / application 53 / infrastructure 58 / app 91），
其中 `jp-app` 的用例只覆盖不依赖 WinUI 运行时的部分（`markdown_edit`、`md_render`、
`PostForm` 映射与预览、
窗口几何/摆放决策、.ico 解析与选档、路径与常量表）——`tests/` 那三个 .NET 测试工程在 Rust 侧没有对应物，
纯逻辑层的用例都并进了各自 crate。

## 为什么是 `winui3` 而不是裸 `windows`

官方 `windows` crate 的元数据来自 Windows SDK，**不含 `Microsoft.UI.Xaml`**——WinAppSDK 的类型
在自己的 `.winmd` 里。所以：

- WinUI 控件树、`Window`、`Application`、`DispatcherQueue` 全部取自
  [`winui3`](https://github.com/Alovchin91/winui3-rs)（windows-rs 生成的 WinAppSDK 投影 + 一层手写组合基座）；
- Win32 与 `Windows.Storage.Pickers` 等非 XAML 的 WinRT 仍走官方 `windows` crate；
- `winui3` 以 git rev 固定引入（`Cargo.toml` 里唯一的非版本依赖），投影与其上游版本绑定。

`jp-app` 启用的 `winui3` 特性都是投影侧的 `#[cfg]` 门控，删掉任何一个都会直接编译失败：
`UI_Composition`（`NavigationViewItem` 整个类都挂在它下面）、`UI_Xaml_Input`（`UIElement::Focus`）、
`UI_Xaml_Controls`/`UI_Xaml_Navigation`（控件与 `Frame` 导航）、`XamlApp`/`XamlApp_Navigation`
（`Application` 组合与自定义页面类型解析）、`MsixDynamicDependency`（未打包运行）、
`UI_Windowing`（`AppWindow`）、`UI_Xaml_Media`（`MicaBackdrop`/`FontFamily`/`SolidColorBrush`）。

## 运行模型

**没有 async runtime。** .NET 侧的 `async/await` + `IAsyncRelayCommand` 换成三条明确规则：

1. 动作（`actions/*`）在 UI 线程被点击触发，立刻 `runtime::spawn_work(work, then)` 派到
   `std::thread`，阻塞式工作（选择器、对话框、文件 IO、AI 请求）都在那儿跑；
2. 工作线程需要用户回答时经 `runtime::ask_ui` 把对话框投回 UI 线程并**阻塞自己**等结果；
   UI 线程注册完 `ContentDialog.ShowAsync` 的完成回调就返回继续跑消息循环——
   只有工作线程在等，不存在「UI 等对话框、对话框等 UI」的死锁；
3. 结果落地后 `post_to_ui` 投回 UI 线程执行 `Refresh`，把状态推回控件。

防抖用 `DispatcherQueueTimer`（`runtime::Debounce`）而非 `Task.Delay`，与原版 `_previewTimer`
同为 150 ms（头信息页）/ 300 ms（正文页）。

**UI 全部由代码构建**：没有 XAML、没有 `x:Bind`、没有 `DataTemplate`、没有 `Style` 与主题资源字典。
每个页面进入时（`OnNavigatedTo`）搭一次控件树并把状态推进去，事件回调把值拉回状态；
两边都是幂等的——程序化 `SetText` 会再触发一次 `TextChanged`，但写回同一个值，不成环。
`Controls ↔ Refresh ↔ 命令条` 之间用 `pages::Slot` 做一层延迟绑定来断开引用环，
页面离开时 `bind(slot, None)`，迟到的工作线程通知就变成空操作。

## 与 .NET 版的差异

功能性差异（都只影响观感，不影响落盘结果）：

| 位置        | .NET 版                                                                       | Rust 版                                                                                     | 原因                                                                                             |
| ----------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| 日期 / 时间 | `CalendarDatePicker` + `TimePicker`                                       | 两个`TextBox`，占位符 `YYYY-MM-DD` / `HH:mm`                                          | 投影里能建，但样式、本地化与 flyout 全要自己撑；解析规则本来就在`PostForm` 里                  |
| 作者选择    | `DropDownButton` + `Flyout` 多选列表                                      | 平铺`CheckBox` 列表 + 一行已选摘要                                                        | 下拉本身在投影里，但 flyout 里的多选列表没有`DataTemplate` 可写                                |
| 正文预览    | `MarkdownTextBlock`（CommunityToolkit）                                     | 自研近似渲染（`md_render` + `markdown_view`）：标题分级、粗斜体/删除线/行内码、链接可点、引用竖条、列表/表格/代码块/分隔线均有形态；图片为灰色 `[图片]` 占位，脚注/HTML 按文本降级 | CommunityToolkit 的控件不在 WinAppSDK 元数据里                                                   |
| 正文工具栏  | `ToggleButton` + `FontIcon`                                               | `CheckBox「预览」`+ 文本字形按钮（`B I ~~ H2 H3 “ </> 代码块 链接 ≡ 1. 表格`）        | `FontIcon` 得手写 `Segoe Fluent Icons` 码点，离线无法校验渲染结果，改用可读的 ASCII/中文标记 |
| 分节容器    | `SettingsCard`                                                              | 圆角`Border` + 小标题                                                                     | 同上，CommunityToolkit 不在投影里                                                                |
| 主题        | 跟随系统深浅色，颜色取主题资源                                                | 固定半透明 ARGB（8%/12% 中性灰、次要文字 78% 不透明）                                       | 代码构建拿不到`TryFindResource`                                                                |
| 标题栏      | `ExtendsContentIntoTitleBar` + 自定义 `TitleBar` + `Assets/AppIcon.ico` | 系统默认标题栏（DWM `USE_IMMERSIVE_DARK_MODE` 转深色，与钉死的深色正文一致），文案「Jekyll 博文工具」（原版字面量是程序集名`JekyllPostTool.App`）；图标见下一行 | 自绘可拖拽区与 caption 命中区成本不抵收益                                      |
| 窗口图标    | `AppWindow.SetIcon("Assets/AppIcon.ico")`（打包资源相对 URI）           | `include_bytes!` 内嵌 `crates/jp-app/assets/AppIcon.ico`，`WM_SETICON` 挂 caption 与任务栏两档，尺寸按 `GetSystemMetricsForDpi` 取 | 未打包 + 代码构建没有 `ms-appx` 解析；原件是 PNG 条目，需先转 DIB（见坑 4）  |
| exe 文件图标 | csproj `<ApplicationIcon>Assets\AppIcon.ico</ApplicationIcon>`          | `crates/jp-app/build.rs` 用 `winresource` 生成 `RT_GROUP_ICON`（同一份 DIB .ico，五帧全带） | rustc 无内建图标支持；资源管理器/固定到开始菜单看的是 exe 里这份资源         |
| 导航项      | `Icon="Folder"/"Contact"/…`                                                | 无图标                                                                                      | 代码构建只能填字形码                                                                             |
| 宽窄布局    | `AdaptiveTrigger`（按物理像素）                                             | `SizeChanged` + 逻辑像素阈值 820                                                          | 与原版一致地用逻辑像素判定，行为见`pages/mod.rs`                                               |

行为差异只有一处：

- **经表单保存会丢掉未识别的 front matter 字段**。ADR-007 承诺 round-trip，`jp-domain` 的
  解析/序列化确实保留了 `unknown_fields`（`jp-infrastructure` 写回时也按序输出）；
  但编辑态 `PostForm` 与 `FrontMatter` 之间的映射只覆盖已知字段，`.NET` 侧
  `BuildFormState().ToFrontMatter()` 同样如此（`PostPageViewModel.Posts.cs:74`）。
  打开一篇带 `image:`/`math:` 的博文再保存，这些字段会消失。
  Rust 版**没有顺手修它**——改了就和 .NET 版行为不一致，无法对照验证。
  该行为由测试 `form_mapping_drops_unknown_front_matter_fields` 钉住（两侧都要修时请一起修）。

另有一处口径曾是两侧共同的缺口，现已同步（2026-10-02）：

- **正文落盘口径**。SRS FR-3.10「保存时正文段原样保留」与
  设计方案 v1.5「正文改动随下一次保存统一写入」在「用户编辑过正文」这一情形下冲突，
  两侧原先都按 FR-3.10 的字面执行——更新路径无视表单正文、从磁盘重切，正文页编辑后点保存会静默丢失。
  现统一为脏标记口径：表单正文仅在内容**真的变化**时置脏（加载/保存后归零），
  保存时只在「新建 或 正文脏」才把表单正文交给
  `PostSaveUseCase::save(..., body: Option<&str>, ...)`，`None` 时更新路径保留磁盘最新 body
  （外部编辑器改动不被覆盖），`Some` 时新建与更新都写这份正文。
  Rust 侧测试 `update_with_body_from_form_overwrites_the_disk_body`、.NET 侧
  `SaveAsync_UpdateWithBodyFromForm_OverwritesTheDiskBody`，两侧均另有界面级端到端正反双向验证。

## 已验证的运行期坑（都已在代码里修掉）

1. **页面离开时不能返回 `E_NOTIMPL`**：上游示例的 `OnNavigatedFrom`/`OnNavigatingFrom` 返回
   `E_NOTIMPL`，单页示例「只进不出」从未触发；本应用真会换页，失败码被 XAML 抛回即整个进程
   `c000027b` fail-fast。`xaml_page!` 宏现按要求返回 `Ok(())`。
2. **`CheckBox::SetIsChecked` 只认标准装箱**：`winui3::Reference::new(bool)` 造的纯 Rust
   `IReference<bool>` 会被 XAML 属性系统拒绝（E_FAIL，错误消息谎报
   `Windows.ApplicationModel.LimitedAccessFeatures`）。`widgets::bool_reference` 改走
   `PropertyValue::CreateBoolean`。
3. **导航回调要用 `InvokedItemContainer()`**：WinUI 3 的 `InvokedItem()` 回吐的是项的
   `Content`（我们的 `TextBlock`），cast 成 `NavigationViewItem` 得 `E_NOINTERFACE`。
4. **`CreateIconFromResourceEx` 在本机不可用**：对 PNG 压缩条目（.NET 那份 .ico 的全部七档）
   和 BMP 条目（连 `C:\Windows\System32\OneDrive.ico` 也一样）都返回 NULL，`GetLastError`
   还不给信息。`icon::build` 因此自己摆像素：`CreateDIBSection`（32bpp 顶向下 BGRA）+
   全零 AND 掩码 `CreateBitmap` + `CreateIconIndirect`，资源也相应转成免解码的 DIB .ico
   （`scripts/gen-icon-asset.py`，逐帧取原生尺寸——原件 16/24/32 是无底色的独立手绘帧）。
   实测挂上的 24/48px（150% 缩放）与 .NET 原件逐像素相同。
5. **文件/目录选择器要绑 Shell 那份 `IInitializeWithWindow`**：`WinRT.Interop.InitializeWithWindow`
   （IID `{00000112-0000-0031-…}`，C# 圈流传的写法）在 `FileOpenPicker`/`FolderPicker` 上
   `QueryInterface` 直接 `E_NOINTERFACE`，`PickXxxAsync` 也就永不显示——症状是「打开已有博文」「导入
   正文」这些按钮点了没反应，只有 `%APPDATA%\JekyllPostTool\crash.log` 留一行
   `文件选择器启动失败: 不支持此接口 (0x80004002)`。改用 `windows::Win32::UI::Shell::IInitializeWithWindow`
   （IID `{3E68D4BD-7135-4D10-8018-9FB6D9F33FA1}`，需开 `Win32_UI_Shell` 特性）即通。
6. **cargo 默认把 exe 链成控制台子系统**：双击 `jp-app.exe` 会先闪一个黑框（.NET WinUI 模板不会，
   rustc 没有对应默认）。`main.rs` 顶部加 `#![windows_subsystem = "windows"]`；
   副作用是 `eprintln!` 无处可看，启动失败的唯一留痕是 `%APPDATA%\JekyllPostTool\crash.log`。
7. **`Command::spawn` 打不开 `.cmd` 垫片，VS Code 按钮「点了没反应」**：PATH 上的 `code` 实为
   `code.cmd`，只有走 Shell（`ShellExecuteW`，即 .NET `UseShellExecute=true` 的路径）才会做
   PATHEXT 解析；`CreateProcess` 直接「系统找不到指定的文件」。`actions::project::launch`
   已改 `ShellExecuteW`（`Win32_UI_Shell`），返回值 ≤32 视为 `SE_ERR_*` 转成 io::Error。
8. **Documents 投影要单独开特性，且 `TextDecorations` 是枚举不是集合**：`Run/Bold/Italic/
   Hyperlink/LineBreak` 与 `TextBlock::Inlines` 全部由 winui3 的 `UI_Xaml_Documents` 特性门控；
   `TextElement::SetTextDecorations` 收的是 `windows::UI::Text::TextDecorations`（u32 标志枚举，
   直接给 `Strikethrough` 即可，不是 UWP 老文档里的集合对象）。另有两个小坑：`Span` 不可激活
   构造（删除线只能挂在摊平后的 `Run` 上）；pulldown-cmark 对紧凑列表项**不发** `Paragraph`
   标签（任务标记同理），`md_render` 用隐式收集器补齐、`TagEnd::Item` 收尾成段落。

## 已知风险

1. **未打包运行需要预装 WindowsAppRuntime，且 exe 旁要放一份 `Microsoft.WindowsAppRuntime.dll`**：
   `winui3` 的 `MsixDynamicDependency` 是静态导入（非 delay-load），缺这份 DLL 进程在 `main` 之前
   就 `0xC0000135` 退出。`PackageDependency::initialize()` 默认探测 2.5（从高到低），
   这份本地 DLL 的版本要与实际解析到的框架包一致——开发时直接从
   `C:\Program Files\WindowsApps\Microsoft.WindowsAppRuntime.<版本>_*\` 拷一份进 `target/debug/`。
   正式分发已接入 `installer/`：MSI 内嵌官方 redist 安装器（安装时 `--quiet` 注册运行时包），
   exe 旁的 DLL 由 `installer/build.ps1` 从同版本 redist 的 Framework MSIX 解出，保证一致。
2. **不产 `resources.pri`**：上游示例为绕开 [WindowsAppSDK#5940](https://github.com/microsoft/WindowsAppSDK/issues/5940)
   在 `Application::Start` 里接 `ResourceManagerRequested` 并指向 `resources.pri`。本项目没有资源包，
   照搬只会得到一个必然取不到资源的空管理器，所以**没有接**。实测五页渲染未撞资源查找异常。
3. `GitHub Actions` 发布管线（`.github/workflows`）与 `installer/` 已同时覆盖 .NET 版与本目录：
   `installer/build.ps1 -Target rust` 产出 `JekyllPostTool_Rust-windows-x64-<版本>.msi`
   （WiX v5，按用户安装，见 `installer/README.md`）。

## 构建

需要 rustup（`stable-x86_64-pc-windows-msvc`）与 Windows SDK（`jp-app` 的 `build.rs` 要调
`rc.exe` 编 exe 图标资源，SDK 路径由 `winresource` 查注册表自动定位），然后：

```powershell
cd src-rs
cargo test                       # 236 个用例，不启动 WinUI
cargo run --bin jp-app           # 必须在桌面会话里跑；Release 用 cargo run --release --bin jp-app
```

打 MSI 安装包（含 WinAppSDK redist 下载与运行时 DLL 提取，前置 `dotnet tool install -g wix`）：

```powershell
powershell -File installer\build.ps1 -Target rust -Version 1.2.0
```

改了 .NET 那份 `AppIcon.ico` 之后重新生成 Rust 侧的图标资源（需要 Python + Pillow）。
这份资源同时喂给运行时（`src/icon.rs` 的窗口图标）与构建期（`build.rs` 的 exe 图标）：

```powershell
python src-rs/scripts/gen-icon-asset.py
```

`build.rs` 走的是 `winresource` 的默认路径，除图标外还会顺带写入版本信息块
（`FileVersion`/`ProductVersion` 取 `CARGO_PKG_VERSION`，`ProductName`/`FileDescription`
取包名 `jp-app`）——与 .NET 侧程序集元数据不同名，但没有行为影响。
