# JekyllPostTool

面向 [Chirpy (Jekyll)](https://github.com/cotes2020/jekyll-theme-chirpy) 博客作者的 Windows 桌面工具，通过 GUI 表单快速初始化与格式化博文文件名及 front matter，支持正文 markdown 编辑、图片插入与 AI 关键字提取，并管理作者信息。

## 功能概览

- **项目管理**：选择博客项目根目录，记忆项目路径；可在资源管理器或 VS Code 中打开项目
- **作者管理**：对 `_data/authors.yml` 进行增删改查，按项目隔离
- **博文头信息**：表单填写 front matter，实时预览文件名与 YAML，支持新建/打开/保存
- **博文正文**：独立页面编辑 markdown 正文、从外部文件导入正文，可插入图片
- **图片插入**：多选本地图片复制到 `assets/img/{slug}/`，markdown 引用插入到正文**光标处**；未保存的博文也可用（slug 由标题生成，中文保留）
- **AI 关键字提取**：配置 OpenAI 兼容 API 后，一键将正文提炼为 tags（替换式）
- **文件名生成**：按 Chirpy 约定 `YYYY-MM-DD-TITLE.md`，中文保留、英文小写、符号压缩
- **冲突处理**：重名时提供修改标题、自动加序号、覆盖三种策略
- **未知字段保留**：保存时原样写回未识别的 front matter 字段

## 技术栈

| 层 | 技术 |
|---|---|
| UI | WinUI 3 (Windows App SDK 2.3)，仿 Windows 设置风格 |
| 框架 | .NET 10, CommunityToolkit.Mvvm (MVVM) |
| YAML | YamlDotNet |
| DI | Microsoft.Extensions.DependencyInjection |
| 架构 | Clean Architecture（Domain → Application → Infrastructure → App） |

## 项目结构

```
src/
├── JekyllPostTool.Domain/          # 领域层：实体、值对象、仓储接口
│   ├── Posts/                       #   Post, FrontMatter, PostValueObjects(Slug/Category/Tag), BodyInsertion
│   ├── Authors/                     #   Author, IAuthorRepository
│   └── Projects/                    #   BlogProject
├── JekyllPostTool.Application/     # 应用层：用例、校验、冲突解决、设置存储
│   ├── JsonFileStore.cs             #   通用 JSON 文件 I/O
│   ├── Posts/                       #   PostCreateUseCase, PostEditUseCase, ImageInserter
│   ├── Authors/                     #   AuthorCrudUseCase
│   ├── Projects/                    #   DefaultProjectSettingService
│   └── Ai/                          #   AiSettings, AiSettingsService
├── JekyllPostTool.Infrastructure/   # 基础设施层：文件系统、YAML 解析、AI 调用
│   ├── Yaml/                        #   YamlFrontMatterParser/Serializer
│   ├── FileSystem/                  #   FilePostRepository, YamlAuthorRepository
│   ├── Import/                      #   MarkdownBodyImporter
│   └── Ai/                          #   OpenAiService
└── JekyllPostTool.App/             # 表示层：WinUI 3 桌面应用
    ├── Pages/                       #   ProjectPage, AuthorsPage, PostPage, PostBodyPage, AdvancedPage
    ├── ViewModels/                  #   PostPageViewModel (+ .Posts/.Body partial), AuthorOption
    └── Services/                    #   DI 服务抽象与 WinUI 实现、TimeZoneFormatter、FrontMatterBuilder
```

## 开发环境

- **OS**：Windows 10 1809+ / Windows 11
- **SDK**：.NET 10 SDK, Windows App SDK 2.3
- **IDE**：Visual Studio 2022 或 TRAE / VS Code
- **架构**：x64（默认），同时支持 x86 / ARM64

## 构建与运行

```powershell
# 还原依赖
dotnet restore JekyllPostTool.slnx

# 构建解决方案
dotnet build JekyllPostTool.slnx

# 运行应用（非打包自包含模式）
dotnet run --project src/JekyllPostTool.App/JekyllPostTool.App.csproj
```

> 应用以非打包（unpackaged）自包含模式运行，无需 MSIX 安装，直接启动 exe 即可。

## 使用指南

### 首次使用

1. 启动应用，左侧导航选择「项目」
2. 点击「选择项目路径」，指定博客项目根目录
3. 可选：点击「资源管理器」或「VS Code」在对应程序中打开项目

### 管理作者

1. 切换到「作者」页
2. 点击「新增作者」填写 id（必填唯一）、name（必填）、twitter、url
3. 选中列表中的作者可编辑或删除

### 新建博文

1. 切换到「博文头信息」页，点击「新建博文」
2. 填写 title、date、categories（最多 2 个）、tags、authors、description
3. 右侧实时预览文件名与 front matter
4. 切换到「博文正文」页编辑正文，或点击「导入正文」从外部 markdown 文件导入
5. 需要配图时：输入 alt（可选）→ 点击「插入图片」→ 多选图片，引用插入到光标处（未保存也可用，目录按标题 slug 生成）
6. 可点击「AI 提取关键字」用 AI 将正文提炼为 tags（需先在高级功能页配置 API）
7. 回到「博文头信息」页点击「保存」，重名时选择处理方式

### 编辑已有博文

1. 点击「打开已有博文」，选择 `_posts/` 下的 .md 文件
2. 表单填充现有 front matter，正文载入「博文正文」页，修改后保存
3. 保存时仅覆盖 front matter 段，正文保留磁盘最新内容（外部编辑器修改不会被覆盖）
4. 若磁盘文件已被外部编辑器修改，保存前会提示

## 配置文件

| 文件 | 路径 | 说明 |
|---|---|---|
| 项目路径 | `%APPDATA%\JekyllPostTool\settings.json` | 记忆上次选择的项目 |
| AI 配置 | `%APPDATA%\JekyllPostTool\ai.json` | OpenAI 兼容 API 的 Base URL / Key / 模型（高级功能页配置） |
| 崩溃日志 | `%APPDATA%\JekyllPostTool\crash.log` | 未处理异常记录 |

## 文档

- [软件需求说明书 (SRS)](docs/SRS.md)
- [设计方案](docs/设计方案.md)
- [界面原型设计](docs/界面原型设计.md)
- [架构决策记录 (ADR)](docs/adr/)
- [术语表](docs/glossary.md)

## 约定

- 文件名：`YYYY-MM-DD-TITLE.md`（Chirpy 约定）
- 文件编码：UTF-8 无 BOM，换行符 LF
- front matter date 格式：`YYYY-MM-DD HH:MM:SS +/-TTTT`
- categories 最多 2 个（主分类、子分类）
- 作者按项目隔离，存于 `_data/authors.yml`
- 未知 front matter 字段 round-trip 保留
