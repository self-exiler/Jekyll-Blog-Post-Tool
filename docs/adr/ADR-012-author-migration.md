# ADR-012: 单数字段 `author` 自动迁移为 `authors` 数组

- 状态: Accepted
- 日期: 2026-07-28

## 上下文

Chirpy/Jekyll 博文 front matter 中，`author`（单数）与 `authors`（复数）都可用于声明作者。SRS v1 最小集选择 `authors` 作为强类型字段。打开旧文件或外部文件时，可能遇到 `author`，需要决定如何处理以避免数据丢失或字段不一致。

## 决定

- v1 强类型字段统一使用 `authors: [...]` YAML 数组。
- 读取博文时，若 front matter 中存在 `author` 字段：
  - 若值为字符串，转为单元素数组 `authors: [value]`。
  - 若值为数组，直接映射为 `authors: value`。
  - 原 `author` 字段不再保留，保存时只输出 `authors`。
- 新建博文只输出 `authors`，不输出 `author`。

## 后果

- 正面：统一作者字段写法，避免同一项目内两种字段混用。
- 正面：与 v1 最小集一致，表单只需要维护一个多选 authors 字段。
- 负面：对于仍依赖 `author` 单数字段的外部工具或主题，保存后格式会改变；但 Chirpy 已支持 `authors`，风险可控。
