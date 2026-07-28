# ADR-009: 技术栈选型

- 状态: Accepted
- 日期: 2026-07-27

## 上下文

需求指定 .NET 10 + WinUI 3。需选定 YAML 库、MVVM 框架、状态持久化方式、打包方式。

## 决定

| 维度 | 选择 | 理由 |
|------|------|------|
| 运行时 | .NET 10 | 需求指定 |
| UI 框架 | WinUI 3 (Windows App SDK) | 需求指定 |
| MVVM | CommunityToolkit.Mvvm | 微软官方，SourceGenerator 驱动，样板代码最少，WinUI 3 标配 |
| YAML 库 | YamlDotNet | .NET 主流，支持有序模型与注释保留，满足 ADR-007 round-trip 硬约束 |
| 状态持久化 | AppData JSON 文件 (`%APPDATA%\JekyllPostTool\`) | 工具级全局状态（RecentProjects 等），跨版本可控，便于调试 |
| 打包 | v1 unpackaged，v2 转 MSIX | v1 优先快速迭代与完全文件访问；v2 转 MSIX + `broadFileSystemAccess` |

## 后果

- 正面：技术栈均为 .NET 生态主流，文档与社区支持充足。
- 正面：YamlDotNet 的有序模型直接支撑 round-trip。
- 负面：unpackaged 需用户自行处理依赖（Windows App SDK 运行时）；转 MSIX 时需补 `Package.appxmanifest` 与能力声明。
- 注意：访问 OneDrive 路径（如 `C:\Users\dioha\OneDrive\文档\self-exiler.github.io`）在 unpackaged 下无限制；MSIX 下需 `broadFileSystemAccess` 能力 + 用户在 Windows 设置中授权文件系统访问。
- 注意：.NET 10 需确认与 Windows App SDK 最新版的兼容性（截至 2026-07，.NET 10 已 GA）。
