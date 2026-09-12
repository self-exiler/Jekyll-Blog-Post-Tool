# JekyllPostTool

面向 [Chirpy (Jekyll)](https://github.com/cotes2020/jekyll-theme-chirpy) 博客作者的 Windows 桌面工具：通过 GUI 表单快速生成与编辑博文文件名及 front matter，支持 markdown 正文编辑、图片插入与 AI 关键字提取，并管理作者信息。

## 功能概览

- **项目管理**：选择博客项目根目录，记忆项目路径；可在资源管理器或 VS Code 中打开项目
- **作者管理**：对 `_data/authors.yml` 进行增删改查，按项目隔离
- **博文头信息**：表单填写 front matter（title/date/时间时区/categories/tags/authors/description），实时预览文件名与最终 YAML；支持新建/打开/保存
- **博文正文**：独立页面编辑 markdown 正文，可从外部文件导入（追加或替换）
- **图片插入**：多选本地图片复制到 `assets/img/{slug}/`，markdown 引用插入到正文**光标处**；未保存的博文也可用（slug 由标题生成，中文保留）
- **AI 关键字提取**：配置 OpenAI 兼容 API 后，一键将正文提炼为 tags（替换式）
- **文件名生成**：按 Chirpy 约定 `YYYY-MM-DD-TITLE.md`，中文保留、英文小写、符号压缩；重名时可选自动加序号或覆盖
- **外部修改检测**：打开与保存基于同一次读盘的内容哈希，磁盘文件被外部修改时保存前会提示
- **未知字段保留**：保存时原样写回未识别的 front matter 字段（round-trip）

## 技术栈

| 层     | 技术                                                                 |
| ------ | -------------------------------------------------------------------- |
| UI     | WinUI 3（Windows App SDK 2.3），仿 Windows 设置风格                  |
| 框架   | .NET 10，CommunityToolkit.Mvvm（MVVM 源生成器）                      |
| YAML   | YamlDotNet                                                           |
| 组合根 | 手写对象装配（`App.xaml.cs`，全部单例，无 DI 容器）                |
| 架构   | Clean Architecture（Domain → Application → Infrastructure → App） |
| 测试   | xUnit（Domain / Application / Infrastructure 三层共 160+ 用例）      |

## 项目结构

```
JekyllPostTool.slnx                 
Directory.Build.props                   # 全局构建属性：Release 下 DebugType=embedded（不产出 .pdb）
docs/
├── SRS.md                              # 软件需求说明书
├── 设计方案.md
├── 界面原型设计.md     
├── glossary.md
├── 单元测试报告.md
└── adr/                                # 12 篇架构决策记录（技术选型、front matter、文件名规则等）

src/
├── JekyllPostTool.Domain/              # 领域层（无外部依赖）
│   ├── Posts/
│   │   ├── Post.cs                     #   博文实体 + 文件名构造/解析（BuildFileName / TryExtractSlug）
│   │   ├── FrontMatter.cs              #   front matter 值对象（已知字段 + 未知字段保序字典）
│   │   ├── PostValueObjects.cs         #   Slug / Category / Tag 值对象
│   │   ├── SlugGenerator.cs            #   标题 → 文件名 slug（中文保留、英文小写、符号压缩）
│   │   ├── MarkdownSplitter.cs         #   markdown 全文切分为 YAML 段与正文段
│   │   ├── BodyInsertion.cs            #   正文指定位置/末尾插入文本的纯函数
│   │   ├── PostContentHash.cs          #   SHA-256 内容哈希（外部修改检测基线）
│   │   └── IPostRepository.cs          #   博文仓储接口 + PostRead（单次读盘快照）
│   ├── Authors/
│   │   ├── Author.cs                   #   作者实体
│   │   └── IAuthorRepository.cs        #   作者仓储接口
│   ├── Projects/
│   │   └── BlogProject.cs              #   博客项目（_posts/ 与 authors.yml 路径派生）
│   └── Common/
│       └── ValidationError.cs          #   字段级校验错误
├── JekyllPostTool.Application/         # 应用层（依赖 Domain）
│   ├── Posts/
│   │   ├── PostSaveUseCase.cs          #   保存/加载用例：校验→文件名→冲突重试→外部修改检测→改名删旧
│   │   ├── PostFormState.cs            #   表单 ↔ front matter 双向映射（唯一权威）
│   │   ├── FrontMatterValidator.cs     #   SRS 字段校验
│   │   ├── FilenameConflictResolver.cs #   文件名冲突检测与候选序号
│   │   ├── PostOperationResult.cs      #   保存结果（Saved/ValidationFailed/Conflict/ModifiedExternally）
│   │   └── TimeZoneFormatter.cs        #   时区偏移 ±hh:mm 格式化/解析/候选列表
│   ├── Authors/
│   │   └── AuthorCrudUseCase.cs        #   作者增删改（读全量→修改→回写）
│   ├── Projects/
│   │   └── DefaultProjectSettingService.cs  # 默认项目路径持久化（settings.json）
│   ├── Ai/
│   │   ├── AiSettings.cs               #   OpenAI 兼容 API 配置
│   │   └── AiSettingsService.cs        #   配置读写（ai.json）
│   └── JsonFileStore.cs                #   通用 JSON 文件 I/O
├── JekyllPostTool.Infrastructure/      # 基础设施层（YamlDotNet）
│   ├── FileSystem/
│   │   ├── FilePostRepository.cs       #   博文仓储：原子写（临时文件+Move）、UTF-8 无 BOM
│   │   ├── YamlAuthorRepository.cs     #   authors.yml 仓储（路径经解析器动态获取）
│   │   └── ImageInserter.cs            #   图片复制到 assets/img/{slug}/ 并生成 markdown 引用
│   ├── Yaml/
│   │   ├── YamlFrontMatterParser.cs    #   YAML → FrontMatter（未知字段转保序字典）
│   │   └── YamlFrontMatterSerializer.cs#   FrontMatter → YAML（未知字段 round-trip）
│   ├── Posts/
│   │   └── PostFileFormat.cs           #   博文文件格式唯一权威（切分/组装/LF 归一）
│   └── Ai/
│       └── OpenAiService.cs            #   Chat Completions 关键字提取
└── JekyllPostTool.App/                 # 表示层（WinUI 3）
    ├── App.xaml / App.xaml.cs          #   入口 + 手写组合根 + 全局异常日志
    ├── MainWindow.xaml(.cs)            #   主窗口（Mica、标题栏、DPI 感知尺寸）
    ├── MainPage.xaml(.cs)              #   NavigationView 导航外壳
    ├── Pages/                          #   五个导航页（xaml + code-behind）
    │   ├── ProjectPage                 #     项目路径选择/打开
    │   ├── AuthorsPage                 #     作者列表与编辑
    │   ├── PostPage                    #     博文头信息表单 + 实时预览（自适应宽窄布局）
    │   ├── PostBodyPage                #     正文编辑/导入/插图
    │   └── AdvancedPage                #     AI 设置
    ├── ViewModels/
    │   ├── PostPageViewModel.cs        #   博文编辑核心（字段/作者/预览防抖）
    │   ├── PostPageViewModel.Posts.cs  #   partial：新建/打开/保存/加载
    │   ├── PostPageViewModel.Body.cs   #   partial：导入正文/插入图片/AI 关键字
    │   ├── AuthorsPageViewModel.cs     #   作者页 VM
    │   ├── ProjectPageViewModel.cs     #   项目页 VM
    │   ├── AdvancedPageViewModel.cs    #   高级功能页 VM
    │   └── AuthorOption.cs             #   作者多选选项（复选框绑定）
    ├── Services/
    │   ├── ProjectContext.cs           #   可观察的当前项目上下文（全应用共享）
    │   ├── WinUIDialogService.cs       #   ContentDialog 封装（含冲突处理选项对话框）
    │   └── WinUIFilePickerService.cs   #   文件/文件夹选择器封装
    ├── Properties/PublishProfiles/FolderProfile.pubxml  # Release 发布配置（自包含）
    ├── app.manifest
    └── Assets/                          #   图标与启动画面资源
Assets/                                 # 图标资源的svg原型
└── build_icons.py                      # 图标生成脚本

tests/
├── JekyllPostTool.Domain.Tests/        #   Post / SlugGenerator / MarkdownSplitter / 值对象
├── JekyllPostTool.Application.Tests/   #   PostSaveUseCase / PostFormState / 校验 / 冲突 / AI 设置等
└── JekyllPostTool.Infrastructure.Tests/#   仓储 / YAML 序列化 / OpenAI 服务

installer/
├── setup.iss                           # Inno Setup 脚本（按用户安装，自包含）
├── build.ps1                           # 发布 + 编译安装包一条龙
├── ChineseSimplified.isl               # 安装器中文语言包（vendor 自上游）
└── README.md                           # 打包与维护说明
```

## 开发环境

- **OS**：Windows 10 1809+ / Windows 11（x64）
- **SDK**：[.NET 10 SDK](https://dotnet.microsoft.com/download/dotnet/10.0)
- **运行时**：Debug 运行依赖系统安装的 Windows App Runtime 2.3+
  （[下载](https://aka.ms/windowsappsdk/2.3/latest/windowsappruntimeinstall-x64.exe)，Release 发布为自包含，无此要求）
- **IDE**：Visual Studio 2022 或 VS Code / Rider

## 构建与运行

```powershell
# 构建解决方案
dotnet build JekyllPostTool.slnx

# 运行应用（Debug，非打包模式）
dotnet run --project src/JekyllPostTool.App/JekyllPostTool.App.csproj

# 运行全部单元测试
dotnet test JekyllPostTool.slnx
```

## 打包与分发

发布为**完全自包含**（.NET 与 WinUI 运行时随应用分发），用 Inno Setup 打成按用户安装的程序：

- 安装位置：`%LOCALAPPDATA%\Programs\JekyllPostTool`
- **无需管理员权限、全程无 UAC、无需联网**，不安装任何系统级依赖
- 产物：单文件 `JekyllPostTool-windows-x64-<版本>.exe`（约 68MB），面向 Windows 10 1809+ x64

```powershell
# 前置：winget install JRSoftware.InnoSetup
powershell -File installer\build.ps1                 # 发布 + 打包
powershell -File installer\build.ps1 -Version 1.2.0  # 指定版本号
powershell -File installer\build.ps1 -SkipPublish    # 复用已有发布产物
```

细节见 [installer/README.md](installer/README.md)。注意：Release 的
`SelfContained` / `WindowsAppSDKSelfContained` 不可改回 false，否则安装包将不内置运行时。

## 使用指南

### 首次使用

1. 启动应用，左侧导航选择「项目」
2. 点击「选择项目路径」，指定博客项目根目录
3. 可选：点击「资源管理器」或「VS Code」在对应程序中打开项目

### 管理作者

1. 切换到「作者」页，点击「新增作者」，填写 id（必填唯一）、name（必填）、twitter、url
2. 选中列表中的作者可编辑或删除

### 新建博文

1. 切换到「博文头信息」页，点击「新建博文」
2. 填写 title、date、categories（主分类 + 子分类，最多 2 个）、tags（空格分隔）、authors、description
3. 右侧实时预览文件名与最终落盘的 front matter
4. 切换到「博文正文」页编辑正文，或「导入正文」从外部 markdown 文件导入
5. 需要配图时：输入 alt（可选）→「插入图片」→ 多选图片，引用插入到光标处
6. 可「AI 提取关键字」将正文提炼为 tags（需先在「高级功能」页配置 API）
7. 回到「博文头信息」页保存；文件重名时选择自动加序号或覆盖

### 编辑已有博文

1. 「打开已有博文」选择 `_posts/` 下的 .md 文件（仅限当前项目目录）
2. 表单填充现有 front matter，正文载入「博文正文」页
3. 保存时仅重写 front matter 段，正文保留磁盘最新内容（外部编辑器的修改不会被覆盖）
4. 若磁盘文件在打开后被外部修改，保存前会提示确认

## 配置与数据文件

| 文件     | 路径                                       | 说明                                     |
| -------- | ------------------------------------------ | ---------------------------------------- |
| 项目路径 | `%APPDATA%\JekyllPostTool\settings.json` | 记忆上次选择的项目                       |
| AI 配置  | `%APPDATA%\JekyllPostTool\ai.json`       | OpenAI 兼容 API 的 Base URL / Key / 模型 |
| 崩溃日志 | `%APPDATA%\JekyllPostTool\crash.log`     | 未处理异常记录（含文件名与行号）         |
| 作者数据 | `<项目>\_data\authors.yml`               | 博客项目内，按项目隔离                   |

## 约定

- 文件名：`YYYY-MM-DD-TITLE.md`（Chirpy 约定），slug 生成中文保留、英文小写
- 文件编码：UTF-8 无 BOM，换行符 LF
- front matter date 格式：`YYYY-MM-DD HH:MM:SS +/-TT:TT`（兼容解析 `+TTTT` 与纯日期）
- 保存以磁盘正文为准：只重写 front matter，正文段原样保留
- 未知 front matter 字段按原顺序 round-trip 保留

## 文档

- [软件需求说明书 (SRS)](docs/SRS.md)
- [设计方案](docs/设计方案.md)
- [界面原型设计](docs/界面原型设计.md)
- [架构决策记录 (ADR)](docs/adr/)（技术选型、front matter 策略、文件名规则等 12 项决策）
- [术语表](docs/glossary.md)
