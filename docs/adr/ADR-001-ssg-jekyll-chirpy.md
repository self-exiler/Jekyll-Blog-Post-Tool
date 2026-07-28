# ADR-001: 采用 Jekyll (Chirpy) 作为唯一目标 SSG

- 状态: Accepted
- 日期: 2026-07-27

## 上下文

原始需求中存在表述矛盾："Jekyll ... 然后通过 hugo 编译"。Jekyll 与 Hugo 是两种互不兼容的静态站点生成器：目录结构（`_posts/` vs `content/`）、front matter 约定、模板语法（Liquid vs 短码）均不同。`docs/` 目录中的两份文档（`write-a-new-post`、`text-and-typography`）描述的是 Chirpy 主题（基于 Jekyll）的格式。

## 决定

以 Jekyll (Chirpy 主题) 为唯一目标 SSG，丢弃 "通过 hugo 编译" 的表述。所有目录约定、front matter 字段、扩展特性以 `docs/` 与 Chirpy 官方文档为准。

## 后果

- 正面：领域模型单一，无需抽象多 SSG 适配层。
- 正面：可深度对齐 Chirpy 扩展字段（prompts、LQIP、mermaid 等）。
- 负面：若将来需要支持 Hugo，需引入格式适配层（当前不在路线图）。
