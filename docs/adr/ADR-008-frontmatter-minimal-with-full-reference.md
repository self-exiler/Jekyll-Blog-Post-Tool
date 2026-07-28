# ADR-008: front matter v1 最小集 + 完整集文档

- 状态: Accepted
- 日期: 2026-07-27

## 上下文

Chirpy front matter 字段繁多（`title`、`date`、`categories`、`tags`、`author`/`authors`、`description`、`image`、`pin`、`toc`、`comments`、`math`、`mermaid`、`render_with_liquid`、`media_subpath` 等）。v1 字段范围直接决定 Post 实体字段集与表单复杂度。

## 决定

v1 实现最小集字段，并附完整集参考文档支撑后续扩展：

### v1 最小集（Post 强类型字段）

| 字段 | 类型 | 约束 |
|------|------|------|
| `title` | string | 必填 |
| `date` | DateTime + 时区偏移 | 必填，格式 `YYYY-MM-DD HH:MM:SS +/-TTTT` |
| `categories` | string[] | 0~2 个 |
| `tags` | string[] | 0~无限，应小写 |
| `authors` | string[] | 引用 `authors.yml` 中的 `author_id`，1~无限 |
| `description` | string | 可选，覆盖自动摘要 |

### 完整集（v2+ 可扩展，v1 round-trip 保留但不编辑）

`author`（单值兼容写法）、`image`（`path`/`alt`/`lqip`）、`pin`、`toc`、`comments`、`math`、`mermaid`、`render_with_liquid`、`media_subpath`。

完整字段定义见 [glossary.md](../glossary.md) 的 front matter 字段参考表。

## 后果

- 正面：v1 表单紧凑，聚焦核心 80% 场景。
- 正面：未知字段通过 ADR-007 round-trip 保留，v1 不实现也不丢失。
- 负面：完整集字段在 v1 无法通过 UI 编辑（需手改文件）。
- 后续：v1.1/v2 可按需将完整集字段逐个升级为强类型字段（无需破坏兼容）。
