# 软件需求说明书 (SRS)

- 项目: Jekyll 博文 markdown 自动生成工具
- 版本: v1.6
- 日期: 2026-09-06
- 状态: Draft

## 1. 引言

### 1.1 目的
本文档定义 Jekyll 博文 markdown 自动生成工具的功能与非功能需求，作为设计、实现与验收的基线。配套的 [设计方案.md](./设计方案.md) 描述"如何实现"，[docs/adr/](./adr/) 记录关键决策的上下文，[docs/glossary.md](./glossary.md) 统一术语。

### 1.2 范围
工具面向 Chirpy (Jekyll) 博客作者，通过 GUI 填表快速初始化与格式化博文文件名和 front matter，提供 markdown 正文编辑（格式化工具栏与实时预览）与图片插入，并管理作者信息。**不**包含：站点构建/部署、Hugo 兼容（见 [ADR-001](./adr/ADR-001-ssg-jekyll-chirpy.md)、[ADR-002](./adr/ADR-002-tool-scope-frontmatter-only.md)）。

### 1.3 术语与缩略语
见 [docs/glossary.md](./glossary.md)。关键术语：BlogProject、Author、Post、Front Matter、Category、Tag、Slug、Round-trip、SSG。

### 1.4 参考资料
- [docs/2019-08-08-write-a-new-post.md](./2019-08-08-write-a-new-post.md)（Chirpy 博文写作规范）
- [docs/2019-08-08-text-and-typography.md](./2019-08-08-text-and-typography.md)（Chirpy 排版示例）
- [Jekyll 官方文档：Posts](https://jekyllrb.com/docs/posts/)
- [Chirpy 主题](https://github.com/cotes2020/jekyll-theme-chirpy)

## 2. 总体描述

### 2.1 产品定位
一款 Windows 桌面工具，将"新建/编辑 Chirpy 博文"从手写文件名与 YAML 转为表单填写，降低格式出错率，提升发布效率。

### 2.2 用户特征
- 单用户本地使用，无多用户协作。
- 熟悉 Jekyll/Chirpy 博客格式，能在外部编辑器（如 VS Code）中编写 markdown 正文。
- 拥有至少一个本地博客项目（含 `_posts/`、`_data/`）。

### 2.3 运行环境
- 操作系统: Windows 10 1809+ / Windows 11
- 运行时: .NET 10, Windows App SDK (WinUI 3)
- 打包: v1 unpackaged（v2 转 MSIX，见 [ADR-009](./adr/ADR-009-tech-stack.md)）
- 文件访问: 需读写本地与 OneDrive 同步目录下的博客项目文件夹

### 2.4 约束
- 仅支持 Jekyll (Chirpy) 格式，不兼容 Hugo。
- 正文编辑以 markdown 源文本为准（非所见即所得富文本编辑；v1.6 起提供格式化工具栏与实时预览，见 FR-4.12~4.13），复杂排版仍可用外部编辑器。
- 作者信息按项目隔离，存于 `_data/authors.yml`。
- 保留 front matter 未知字段（round-trip）。
- 遵守 Chirpy 约定：文件名 `YYYY-MM-DD-TITLE.md`、categories ≤ 2。
- 输出文件编码：UTF-8 无 BOM，换行符 LF（见 [ADR-010](./adr/ADR-010-file-encoding.md)）。

### 2.5 假设与依赖
- 用户的博客项目已符合 Chirpy 目录结构（`_posts/`、`_data/authors.yml` 可不存在时由工具创建）。
- `.NET 10` 与 `Windows App SDK` 运行时已安装或随工具分发。

## 3. 功能需求

### 3.1 博客项目管理

| 编号 | 需求 | 优先级 |
|------|------|--------|
| FR-1.1 | 用户可通过文件夹选择器选择一个本地博客项目根目录 | P0 |
| FR-1.2 | ~~选择后工具校验目录结构：存在 `_posts/` 视为有效项目；缺失时提示并询问是否创建~~（v1.2 移除：工具不再进行项目校验，保存博文时自动创建缺失的 `_posts/` 目录） | — |
| FR-1.3 | 工具记忆一个默认项目路径，启动时自动进入；路径失效时静默清空并回到选择页 | P0 |
| FR-1.4 | 默认项目路径持久化于 AppData JSON，应用重启后保留 | P0 |
| FR-1.5 | BlogProject 本身无状态，不存项目级默认值 | P0 |
| FR-1.6 | 用户可在文件资源管理器中打开当前项目根目录 | P0 |
| FR-1.7 | 用户可在 VS Code 中打开当前项目根目录；VS Code 未安装或不在 PATH 时提示用户 | P0 |

依据: [ADR-005](./adr/ADR-005-blogproject-stateless-recent-global.md)

### 3.2 作者管理

| 编号 | 需求 | 优先级 |
|------|------|--------|
| FR-2.1 | 列出当前项目 `_data/authors.yml` 中所有作者（id、name、twitter、url） | P0 |
| FR-2.2 | 新增作者：填入 id（必填唯一）、name（必填）、twitter（可选）、url（可选），写回 `authors.yml` | P0 |
| FR-2.3 | 编辑现有作者字段，保存写回 | P0 |
| FR-2.4 | 删除作者：确认后直接从 `authors.yml` 移除，不扫描博文引用 | P0 |
| FR-2.5 | 若 `_data/authors.yml` 不存在，首次新增作者时创建文件 | P0 |
| FR-2.6 | `authors.yml` 保留注释与键顺序（round-trip，见 ADR-007） | P1 |

依据: [ADR-003](./adr/ADR-003-author-per-project-isolation.md)、[ADR-004](./adr/ADR-004-freeform-categories-tags.md)

### 3.3 博文管理（博文头信息页）

| 编号 | 需求 | 优先级 |
|------|------|--------|
| FR-3.1 | 新建博文：打开空白表单（无预填默认值，date 为空），填写 v1 最小集字段 | P0 |
| FR-3.2 | 打开已有博文：通过文件选择器（限定当前项目 `_posts/`）或拖放当前项目 `_posts/` 内 .md 文件 | P0 |
| FR-3.3 | 编辑 front matter 字段（见 §5.1 v1 最小集） | P0 |
| FR-3.4 | 文件名自动生成：date 取 front matter `date` 的 `YYYY-MM-DD`；TITLE 从 `title` slugify（中文原样、英文小写、空格转 `-`、连续空格/符号压缩为单个 `-`、去除非法字符） | P0 |
| FR-3.5 | 文件名仅作为预览，不提供给用户手动编辑；任何 title 变更都会重新生成文件名 | P0 |
| FR-3.6 | 重名冲突：保存时若 `_posts/` 下已存在同名文件，提示并提供选项——改 title / 自动加序号后缀 / 覆盖（二次确认） | P0 |
| FR-3.7 | 保留未知 front matter 字段：保存时将未知字段原样写回，不丢失 | P0 |
| FR-3.8 | 多作者：`authors` 字段支持多选（引用 `authors.yml` 中的 id），输出为 YAML 数组；**作者可留空**（v1.2：由必填改为可选，为空时省略 `authors` 字段，与 categories/tags 一致）；读取时若存在单数字段 `author`，自动迁移为 `authors` | P0 |
| FR-3.9 | categories 最多 2 个，第一个为主分类、第二个为子分类，超出时 UI 阻止输入；tags 无上限，原样保留大小写与中英文 | P0 |
| FR-3.10 | 保存时仅覆盖 front matter 段，正文段原样保留（即便被外部编辑器修改） | P0 |
| FR-3.11 | 若打开后外部编辑器修改了磁盘文件，保存前提示用户文件已变更并询问是否继续 | P0 |

依据: [ADR-002](./adr/ADR-002-tool-scope-frontmatter-only.md)、[ADR-006](./adr/ADR-006-filename-slugify-chinese-preserved.md)、[ADR-007](./adr/ADR-007-unknown-field-roundtrip-import-body-only.md)、[ADR-008](./adr/ADR-008-frontmatter-minimal-with-full-reference.md)、[ADR-011](./adr/ADR-011-filename-generation.md)、[ADR-012](./adr/ADR-012-author-migration.md)

### 3.4 博文正文

"博文正文"为左侧导航独立页面（v1.5），提供正文 markdown 编辑、正文导入与光标处图片插入；v1.6 起编辑区升级为 Markdown 编辑器：格式化工具栏 + 可开关的实时预览分栏（FR-4.12~4.13）。

| 编号 | 需求 | 优先级 |
|------|------|--------|
| FR-4.1 | 从外部 markdown 文件导入正文：剥离首个 `---` front matter 块，仅取正文部分 | P0 |
| FR-4.2 | 导入的 front matter 不合并到当前表单，忽略 | P0 |
| FR-4.3 | 导入后正文默认追加到目标博文文件正文段；提供选项改为替换 | P1 |
| FR-4.4 | 在"博文正文"页直接编辑 markdown 正文（v1.5 新增；保存时仍只覆盖 front matter 段、保留磁盘最新 body，见 FR-3.10） | P0 |
| FR-4.5 | 插入图片：通过文件选择器多选本地图片（jpg/jpeg/png/gif/webp/svg），生成的 markdown 引用插入到正文编辑器**光标处** | P0 |
| FR-4.6 | 图片复制到当前博文资源目录 `/assets/img/{slug}/`；`{slug}` 取自已保存文件名（去日期前缀），未保存时用 `title` 即时生成（中文原样保留，见 ADR-006） | P0 |
| FR-4.7 | 目标文件名保留原文件名；目标路径已存在同名文件时自动追加 `-1`、`-2` 序号直到可用 | P0 |
| FR-4.8 | 源文件以复制方式导入（非移动），原文件不受影响 | P0 |
| FR-4.9 | 每张图片生成标准 markdown 引用 `![alt](/assets/img/{slug}/{filename})`；光标无效（无焦点/超出范围）时追加到正文末尾 | P0 |
| FR-4.10 | alt 文本默认为空；用户可在插入前在输入框填写统一 alt（应用于本次所有图片） | P1 |
| FR-4.11 | 未保存的博文也可插入图片：slug 由 title 生成（与文件名预览一致），标题为空时禁用插入按钮 | P0 |
| FR-4.12 | Markdown 格式化工具栏（v1.6 新增）：粗体 `**`、斜体 `*`、删除线 `~~`、行内代码 `` ` ``、代码块 ``` 围栏、二级/三级标题 `##`/`###`、引用 `>`、无序/有序列表 `-`/`1.`、链接 `[文本](url)`、表格模板。包裹类命令作用于选区（两侧已有相同标记时解除包裹；无选区时插入成对标记并居中光标）；行前缀类命令作用于光标/选区覆盖的每一行（首行已有前缀时改为移除）；操作后焦点回到正文编辑框 | P1 |
| FR-4.13 | Markdown 实时预览（v1.6 新增）：正文编辑框旁提供预览分栏，随正文输入防抖刷新渲染；可用开关隐藏/显示（默认开启，隐藏时编辑框占满全宽）。渲染基于 Markdig（支持表格、任务列表、删除线、自动链接），跟随应用深浅色主题；预览为只读展示，不参与保存内容（保存的仍是 markdown 源文本） | P1 |

依据: [ADR-007](./adr/ADR-007-unknown-field-roundtrip-import-body-only.md)、[ADR-006](./adr/ADR-006-filename-slugify-chinese-preserved.md)、[Chirpy 媒体约定](./2019-08-08-write-a-new-post.md#media)

### 3.5 全局设置

| 编号 | 需求 | 优先级 |
|------|------|--------|
| FR-5.1 | 项目选择页展示并允许修改项目路径（v1.2：原"默认项目路径"重命名为"项目路径"，"更改默认路径"按钮重命名为"选择项目路径"） | P0 |

### 3.6 高级功能（v1.5：仅含 AI 设置）

高级功能页承载扩展性工具集。v1.3 曾含"插入图片"（FR-6.x）；v1.5 起图片插入移至"博文正文"页（见 §3.4 FR-4.5~4.11），高级功能页保留"AI 设置"卡片与"更多功能"占位。

### 3.7 AI 关键字提取（v1.4 新增，v1.5 起为高级功能页主要功能）

配置 OpenAI 兼容 API 后，将当前博文正文发送给 AI 提炼关键字并导入标签。

| 编号 | 需求 | 优先级 |
|------|------|--------|
| FR-7.1 | 在"高级功能"页提供"AI 设置"卡片，配置 Base URL、API Key、模型名；持久化到 `%APPDATA%/JekyllPostTool/ai.json` | P0 |
| FR-7.2 | API Key 以 PasswordBox 录入（掩码显示）；保存与清空按钮 | P0 |
| FR-7.3 | 在"博文头信息"页"标签"卡片提供"AI 提取关键字"按钮；点击后将正文发送给 AI 提炼最多 5 个关键字 | P0 |
| FR-7.4 | 提取的关键字**替换**已有标签（非追加），用英文逗号分隔填入 tags 输入框 | P0 |
| FR-7.5 | 提取期间按钮禁用，避免重复点击；正文为空或未配置时给出明确提示 | P1 |
| FR-7.6 | 兼容中英文逗号、换行、顿号分隔的返回结果；自动去除序号前缀（如 `1.`、`1、`）；大小写不敏感去重 | P1 |
| FR-7.7 | 调用失败（网络错误、非 2xx、未配置）时弹对话框显示错误信息，不破坏现有标签 | P0 |

依据: OpenAI Chat Completions API 兼容协议（`POST {base_url}/v1/chat/completions`，Bearer 鉴权）

## 4. 非功能需求

| 类别 | 需求 |
|------|------|
| 性能 | 打开项目 ≤ 1s；打开单篇博文 front matter ≤ 200ms；保存 ≤ 300ms |
| 可用性 | 表单字段在保存时统一校验（必填、格式、长度）；错误提示明确指向具体字段 |
| 可靠性 | 写文件前先写入临时文件，写成功后原子替换；异常时不损坏用户文件 |
| 可维护性 | 领域层与基础设施层分离；YAML 读写集中于一处；front matter 字段定义集中可扩展 |
| 兼容性 | 支持 OneDrive 同步路径；支持中文文件名与路径 |
| 国际化 | v1 仅中文 UI，字符串外置以便后续 i18n |
| 安全 | 不上传任何数据；不执行博文中的脚本；文件操作限于用户选择的项目目录内 |

## 5. 数据需求

### 5.1 front matter 字段（v1 最小集）

| 字段 | 类型 | 约束 | 示例 |
|------|------|------|------|
| `title` | string | 必填 | `Writing a New Post` |
| `date` | DateTime + 时区偏移 | 必填，格式 `YYYY-MM-DD HH:MM:SS +/-TTTT` | `2019-08-08 14:10:00 +0800` |
| `categories` | string[] | 0~2 个，[主分类, 子分类] | `[Blogging, Tutorial]` |
| `tags` | string[] | 0~无限，原样保留 | `[Writing]` |
| `authors` | string[] | 0~无限，引用 `authors.yml` 的 id；可留空（v1.2） | `[cotes]` 或 `[]` |
| `description` | string | 可选，多行，无最大长度 | `Short summary.` |

完整集（v2+ 扩展，v1 round-trip 保留）：`author`、`image`、`pin`、`toc`、`comments`、`math`、`mermaid`、`render_with_liquid`、`media_subpath`。详见 [glossary.md](./glossary.md)。

### 5.2 文件路径约定

| 路径 | 含义 |
|------|------|
| `_posts/YYYY-MM-DD-TITLE.md` | 博文文件 |
| `_data/authors.yml` | 作者信息 |
| `_config.yml` | 站点配置（工具只读不写） |
| `assets/img/{slug}/` | 博文图片资源目录（v1.3 起提供；v1.5 起由"博文正文"页插入图片功能使用） |

### 5.3 持久化数据

| 数据 | 位置 | 格式 |
|------|------|------|
| 默认项目路径 | `%APPDATA%\JekyllPostTool\settings.json` | JSON |
| AI 配置（v1.4） | `%APPDATA%\JekyllPostTool\ai.json` | JSON |

```json
{
  "defaultProjectPath": "C:\\Users\\dioha\\OneDrive\\文档\\self-exiler.github.io"
}
```

ai.json 结构（v1.4）：

```json
{
  "baseUrl": "https://api.openai.com/v1",
  "apiKey": "sk-...",
  "model": "gpt-4o-mini"
}
```

依据: [ADR-005](./adr/ADR-005-blogproject-stateless-recent-global.md)、[ADR-009](./adr/ADR-009-tech-stack.md)

## 6. 典型用例

### UC-1: 新建并发布一篇博文
1. 用户启动工具，自动进入默认项目（或先选择项目文件夹）。
2. 点击"新建博文"，在"博文头信息"页填写 title、date、categories、tags、authors、description。
3. 工具实时预览生成的文件名与 front matter。
4. 在"博文正文"页用 Markdown 编辑器编辑正文（格式化工具栏、实时预览），导入外部 markdown 追加正文，或多选图片插入到光标处（未保存时也可用，slug 由 title 生成）。
5. 点击保存：工具校验 → 检查重名 → 写入 `_posts/YYYY-MM-DD-TITLE.md`（front matter 与正文一次写入）。
6. 可配置 AI 后用"AI 提取关键字"按钮将正文提炼为 tags。

### UC-2: 修改已有博文的 front matter
1. 点击"打开博文"，通过文件选择器选择当前项目 `_posts/` 下的博文，或从资源管理器拖放文件到工具。
2. 工具解析 front matter 填入"博文头信息"表单（未知字段保留但不展示），正文载入"博文正文"页。
3. 用户修改字段（如改 categories、加 authors）。
4. 保存：若磁盘文件已被外部修改则提示用户；否则仅覆盖 front matter 段，正文原样保留。

### UC-3: 管理作者
1. 打开作者管理面板，看到 `authors.yml` 中所有作者。
2. 新增或编辑作者（id/name/twitter/url）。
3. 删除作者 → 确认后直接移除，不扫描引用。

## 7. 验收标准（摘要）

- 能选择 Chirpy 博客项目，项目路径跨会话保留，失效时静默清空；不在选择时进行 `_posts/` 校验（保存时自动创建）。
- 能在文件资源管理器或 VS Code 中打开当前项目。
- 能对 `_data/authors.yml` 增删改，删除时不扫描引用。
- 能新建博文并自动生成符合 `YYYY-MM-DD-TITLE.md` 的文件名（含中文，连续符号压缩）。
- 能打开已有博文、编辑 front matter、保留未知字段、仅覆盖 front matter 段。
- 重名冲突提供三选项处理。
- 多作者以 `authors: [...]` 数组形式输出，**作者为空时省略 `authors` 字段**（与 categories/tags 一致）；单数字段 `author` 自动迁移。
- 导入正文时正确剥离外部文件的 front matter，默认追加；"博文正文"页可直接编辑 markdown 正文（FR-4.4）。
- "博文正文"页提供 Markdown 格式化工具栏（作用于选区/行，可解除包裹与移除前缀）与可开关的实时预览分栏；保存内容仍为 markdown 源文本（FR-4.12~4.13）。
- 输出文件编码为 UTF-8 无 BOM，换行符 LF。
- "博文正文"页提供插入图片：多选本地图片复制到 `/assets/img/{slug}/`，markdown 引用插入到正文光标处（无光标时追加末尾），文件名冲突自动加序号（FR-4.5~4.11）。
- 高级功能页提供"AI 设置"卡片，可配置 OpenAI 兼容 API 并用"AI 提取关键字"生成 tags（FR-7.x）。
