# ADR-006: 文件名从 title/date 生成，中文原样保留

- 状态: Accepted
- 日期: 2026-07-27

## 上下文

Chirpy 要求文件名 `YYYY-MM-DD-TITLE.md`，且文件名 date 应与 front matter date 一致以避免排序错乱。TITLE 可手输或从 title 自动生成；中文标题的 slugify 策略需明确。

## 决定

- 文件名 date 取自 front matter 的 `date` 字段（格式 `YYYY-MM-DD`）。
- 文件名 TITLE 从 front matter 的 `title` 字段自动 slugify，规则如下：
  - 中文原样保留（不转拼音）。
  - 英文字母转小写。
  - 空格转连字符 `-`。
  - 去除文件系统非法字符（`\ / : * ? " < > |`）。
  - 示例：`title = "用 WinUI3 制作博客工具"` → 文件名 `2026-07-27-用-winui3-制作博客工具.md`。
- 文件名生成后允许用户在保存前手动修改。
- **重名冲突处理**：保存时若 `_posts/` 下已存在同名文件，提示并给选项——改 title / 自动加序号后缀（`-2`、`-3`） / 覆盖（覆盖需二次确认）。

## 后果

- 正面：文件名与 front matter date 强一致，避免 Chirpy 排序问题。
- 正面：中文保留对作者友好，无需拼音库依赖。
- 负面：中文文件名对部分旧系统/URL 不友好（可接受，现代环境普遍支持）。
- 负面：重名冲突的三选项 UI 略复杂，但保障安全。
