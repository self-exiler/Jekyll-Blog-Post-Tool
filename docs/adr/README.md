# 架构决策记录 (ADR)

本目录记录 Jekyll 博文 markdown 自动生成工具的架构决策。每条 ADR 描述一个决策的上下文、决定与后果，供后续查阅、复审与变更追踪。

## 决策列表

| 编号 | 标题 | 状态 |
|------|------|------|
| [ADR-001](./ADR-001-ssg-jekyll-chirpy.md) | 采用 Jekyll (Chirpy) 作为唯一目标 SSG | Accepted |
| [ADR-002](./ADR-002-tool-scope-frontmatter-only.md) | 工具边界限定为 front matter 编辑 | Accepted |
| [ADR-003](./ADR-003-author-per-project-isolation.md) | 作者信息按项目隔离 | Accepted |
| [ADR-004](./ADR-004-freeform-categories-tags.md) | 分类与标签自由输入 | Accepted |
| [ADR-005](./ADR-005-blogproject-stateless-recent-global.md) | BlogProject 无状态，默认项目路径全局持久化 | Accepted |
| [ADR-006](./ADR-006-filename-slugify-chinese-preserved.md) | 文件名从 title/date 生成，中文原样保留 | Accepted |
| [ADR-007](./ADR-007-unknown-field-roundtrip-import-body-only.md) | 保留未知字段 round-trip，导入仅取正文 | Accepted |
| [ADR-008](./ADR-008-frontmatter-minimal-with-full-reference.md) | front matter v1 最小集 + 完整集文档 | Accepted |
| [ADR-009](./ADR-009-tech-stack.md) | 技术栈选型 | Accepted |
| [ADR-010](./ADR-010-file-encoding.md) | 博文文件统一使用 UTF-8 无 BOM + LF 编码 | Accepted |
| [ADR-011](./ADR-011-filename-generation.md) | 文件名从 title/date 自动生成且始终跟随 title | Accepted |
| [ADR-012](./ADR-012-author-migration.md) | 单数字段 `author` 自动迁移为 `authors` 数组 | Accepted |

## 相关文档

- [词汇表 / 领域模型](../glossary.md)
- [Chirpy 原始文档](../2019-08-08-write-a-new-post.md)
