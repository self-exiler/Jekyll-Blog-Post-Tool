# 安装程序（installer/）

用 Inno Setup 打包的 Windows 安装程序。应用发布为**完全自包含**
（.NET 10 与 Windows App SDK 运行时随应用分发，见 `JekyllPostTool.App.csproj`），
因此安装程序轻装上阵：

- 安装位置：`%LOCALAPPDATA%\Programs\JekyllPostTool`（仅当前用户，不写 Program Files）
- **无需管理员权限，全程无 UAC**，无需联网，不安装任何系统级依赖
- 卸载同样无需提权（控制面板"应用和功能"或开始菜单卸载入口）
- 面向 x64，最低 Windows 10 1809

## 构建

```powershell
# 前置：winget install JRSoftware.InnoSetup
powershell -File installer\build.ps1                 # 发布 + 打包
powershell -File installer\build.ps1 -Version 1.2.0  # 指定版本号
powershell -File installer\build.ps1 -SkipPublish    # 复用已有发布产物
```

产物：`installer/output/JekyllPostTool-windows-x64-<版本>.exe`，命名规则为 **程序名-操作系统-cpu架构-版本号**（静默安装：追加 `/VERYSILENT /NORESTART`）。

## 维护注意

- **不要把 Release 的 `SelfContained` / `WindowsAppSDKSelfContained` 改回 false**：
  安装器不装任何运行时，改回框架依赖会让应用在干净机器上无法启动。
  （Debug 配置保持框架依赖，依赖系统安装的运行时。）
- 若将来想改回"框架依赖 + 安装器补装运行时"的小体积方案，注意：
  .NET 与 VC++ 的官方安装器只支持机器级安装（需要管理员），
  WinAppSDK 2.x 的 AppX 包名为 `Microsoft.WindowsAppRuntime.2`（小版本在包版本号里）。
