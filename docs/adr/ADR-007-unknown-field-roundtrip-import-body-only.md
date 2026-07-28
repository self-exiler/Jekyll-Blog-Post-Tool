# ADR-007: 保留未知字段 round-trip，导入仅取正文

- 状态: Accepted
- 日期: 2026-07-27

## 上下文

打开已有博文时，其 front matter 可能含 v1 不认识的字段（`image`、`math`、`lqip`、`mermaid` 等）。保存时如何处理这些未知字段？另外，从外部 markdown 文件导入正文时，导入文件可能也带 front matter。

## 决定

- **未知字段保留 (round-trip)**：工具解析 front matter 时，将已知字段映射到强类型模型，未知字段保留在有序字典中；保存时先写已知字段，再写未知字段，原样写回。这是 YAML 库选型的硬约束（见 ADR-009 选 YamlDotNet）。
- **导入正文**：从 markdown 文件导入时，仅取正文部分（剥离首个 `---` 包裹的 front matter 块），忽略导入文件自身的 front matter，不合并到当前表单。

## 后果

- 正面：round-trip 保证不丢失用户手动添加的高级字段，工具可安全用于已有丰富 front matter 的博文。
- 正面：导入逻辑简单（切掉首个 `---` 块即可），无需 markdown 解析库。
- 负面：round-trip 要求 YAML 库支持有序模型与注释保留，排除了 SharpYaml 与自写解析器。
- 注意：编辑 front matter 期间若正文被外部编辑器修改，保存时工具仅覆盖 front matter 段，正文段原样保留（按行切分重组）。
