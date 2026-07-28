# ADR-010: 博文文件统一使用 UTF-8 无 BOM + LF 编码

- 状态: Accepted
- 日期: 2026-07-28

## 上下文

工具需要读写 Jekyll/Chirpy 博文 markdown 文件。Windows 平台默认文本编码/换行与 Jekyll 生态（Git、GitHub Pages、Linux 构建环境）存在差异，需要明确输出文件的编码与换行符策略。

## 决定

- 所有由工具写入的 markdown 文件统一使用 **UTF-8 无 BOM** 编码。
- 所有由工具写入的 markdown 文件统一使用 **LF（`\n`）** 换行符。
- 读取文件时按 UTF-8 解码；切分 front matter 与 body 时同时兼容 LF 与 CRLF 输入，兼容用户已有文件。

## 后果

- 正面：与 Jekyll/Chirpy 示例、GitHub Pages 构建环境保持一致，减少跨平台构建差异。
- 正面：避免 Windows 记事本对 UTF-8 无 BOM 的历史兼容问题已不是现代环境主要痛点。
- 负面：若用户后续用某些旧版 Windows 工具打开可能显示乱码或缺少换行，但可用现代编辑器规避。
- 负面：现有 CRLF 文件在首次保存后会整体转为 LF，Git diff 可能显示全文件变更；属于一次性迁移成本。
