# 词汇表 (Glossary)

本词汇表统一 Jekyll 博文工具的领域术语，供 ADR、代码与沟通使用。术语一旦定义，各方应严格遵守其含义。

## 核心领域术语

### BlogProject（博客项目）
指一个 Jekyll/Chirpy 博客的本地根目录（如 `C:\Users\dioha\OneDrive\文档\self-exiler.github.io`）。在工具中是**无状态的文件夹引用**——仅记录路径，不携带配置。包含 `_posts/`、`_data/authors.yml`、`_config.yml` 等约定子路径。见 [ADR-005](./adr/ADR-005-blogproject-stateless-recent-global.md)。

### DefaultProjectPath（默认项目路径）

**工具级全局状态**，记录用户常用的单个 BlogProject 路径，启动时自动进入；路径失效时静默清空。存于 AppData JSON。独立于任何项目。见 [ADR-005](./adr/ADR-005-blogproject-stateless-recent-global.md)。

### Author（作者）
BlogProject 聚合内的实体，存于 `_data/authors.yml`。字段：
- `id`（YAML 键，如 `cotes`）——博文通过此 id 引用
- `name`（全名）
- `twitter`（可选，Twitter handle）
- `url`（可选，作者主页）

按项目隔离，见 [ADR-003](./adr/ADR-003-author-per-project-isolation.md)。

### Post（博文）
BlogProject 聚合内的实体，存于 `_posts/YYYY-MM-DD-TITLE.md`。由**文件名 + front matter + 正文**三部分组成。工具仅管理文件名与 front matter，正文交外部编辑器。见 [ADR-002](./adr/ADR-002-tool-scope-frontmatter-only.md)。

### Front Matter（前置元数据）
博文 markdown 文件首部 `---` 包裹的 YAML 块，描述博文的元数据。Chirpy 约定 `layout` 默认为 `post`，无需显式声明。

### Category（分类）
Post 的值对象，front matter 中 `categories: [TOP, SUB]`，最多 2 个。第一个为主分类（TOP），第二个为子分类（SUB）。自由输入，无受控词库。见 [ADR-004](./adr/ADR-004-freeform-categories-tags.md)。

### Tag（标签）
Post 的值对象，front matter 中 `tags: [t1, t2, ...]`，0~无限。自由输入，无受控词库。工具原样保留大小写与中英文，不做小写强制。见 [ADR-004](./adr/ADR-004-freeform-categories-tags.md)。

### Slug / Slugify（文件名生成）
将 title 转为文件名安全字符串的过程。本工具规则：中文原样保留、英文小写、空格转连字符、连续空格/符号压缩为单个连字符、去除文件系统非法字符、去除首尾连字符。见 [ADR-006](./adr/ADR-006-filename-slugify-chinese-preserved.md)、[ADR-011](./adr/ADR-011-filename-generation.md)。

### Round-trip（往返保留）
打开博文 → 编辑 front matter → 保存时，未知字段（工具不认识的 front matter 键）原样保留写回，不丢失。见 [ADR-007](./adr/ADR-007-unknown-field-roundtrip-import-body-only.md)。

### SSG（Static Site Generator，静态站点生成器）
将 markdown + 模板编译为静态 HTML 的工具。本工具目标 SSG 为 Jekyll（Chirpy 主题），不兼容 Hugo。见 [ADR-001](./adr/ADR-001-ssg-jekyll-chirpy.md)。

## 文件路径约定

| 路径 | 含义 |
|------|------|
| `_posts/` | 博文 markdown 存放目录 |
| `_posts/YYYY-MM-DD-TITLE.md` | 博文文件名格式 |
| `_data/authors.yml` | 作者信息 YAML |
| `_config.yml` | Jekyll 站点配置（含 `timezone`、`toc`、`comments.provider` 等全局设置） |

## front matter 字段参考

### v1 最小集（工具强类型支持）

| 字段 | 类型 | 示例 | 说明 |
|------|------|------|------|
| `title` | string | `Writing a New Post` | 博文标题，必填 |
| `date` | datetime+tz | `2019-08-08 14:10:00 +0800` | 发布日期 + 时区，必填 |
| `categories` | string[] | `[Blogging, Tutorial]` | 0~2 个 |
| `tags` | string[] | `[Writing]` | 0~无限，原样保留大小写 |
| `authors` | string[] | `[cotes, jane]` | 引用 `authors.yml` 的 id |
| `description` | string | `Short summary.` | 可选，覆盖自动摘要 |

### 完整集（v2+ 扩展，v1 round-trip 保留但不编辑）

| 字段 | 类型 | 说明 |
|------|------|------|
| `author` | string\|string[] | 单作者兼容写法；读取时自动迁移为 `authors`，保存时只输出 `authors` |
| `image` | string\|object | 预览图，子字段 `path`/`alt`/`lqip` |
| `pin` | bool | 置顶博文 |
| `toc` | bool | 关闭目录 |
| `comments` | bool | 关闭评论 |
| `math` | bool | 启用 MathJax |
| `mermaid` | bool | 启用 Mermaid 图表 |
| `render_with_liquid` | bool | 禁用 Liquid 渲染 |
| `media_subpath` | string | 资源路径前缀 |

## 领域模型概览

```
BlogProject (无状态文件夹引用)
├── Author[]            ← _data/authors.yml
│   └── id, name, twitter, url
└── Post[]              ← _posts/*.md
    ├── Filename        ← YYYY-MM-DD-TITLE.md (从 title/date 自动生成, ADR-006)
    ├── FrontMatter
    │   ├── title, date, description       ← 强类型 (v1 最小集)
    │   ├── categories: Category[] (0~2)   ← 值对象
    │   ├── tags: Tag[] (0~∞)              ← 值对象
    │   ├── authors: Author.id[] (1~∞)     ← 引用
    │   └── <unknown fields>               ← round-trip 保留 (ADR-007)
    └── Body             ← 外部编辑器编辑, 工具不管理 (ADR-002)

DefaultProjectPath (工具级全局状态, AppData JSON)
└── string              ← BlogProject 路径
```

## 聚合边界小结

- **BlogProject 聚合根**：包含 Author[] 与 Post[]。跨项目不共享。
- **Author**：聚合内实体，Post 通过 `authors: [id]` 引用其 id。
- **Post**：聚合内实体，文件名与 front matter 是其不变量。
- **Category / Tag**：Post 内的值对象，无独立身份。
- **DefaultProjectPath**：独立于 BlogProject 聚合，属工具级配置聚合。
