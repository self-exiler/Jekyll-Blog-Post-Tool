# 安装程序（installer/）

用 **WiX Toolset v5** 构建的 Windows MSI 安装包，同时覆盖两个发行版：

| 产物 | 定义文件 | 内容 |
| --- | --- | --- |
| `JekyllPostTool-windows-x64-<版本>.msi` | `JekyllPostTool.wxs` | .NET 版（`src/`），发布为**完全自包含**，MSI 不装任何运行期依赖 |
| `JekyllPostTool_Rust-windows-x64-<版本>.msi` | `JekyllPostTool.Rust.wxs` | Rust 版（`src-rs/`），内嵌 WinAppSDK 2.5.1 redist，安装时静默注册运行时 |

两个包均为**按用户安装**（MSI `Scope="perUser"`）：

- 安装位置：`%LOCALAPPDATA%\Programs\JekyllPostTool`（Rust 版为 `JekyllPostTool-rust`）
- **无需管理员权限，全程无 UAC**；卸载同样无需提权（设置→应用 或开始菜单卸载入口）
- 安装向导为中文（WixUI，`-culture zh-CN`），可勾选"桌面快捷方式"特性（默认不勾选，对齐原 Inno 的 desktopicon 任务），完成页可勾选"运行"
- 面向 x64，最低 Windows 10 1809

## 构建

```powershell
# 前置：dotnet tool install -g wix（5.x；UI 扩展缺失时 build.ps1 自动安装）
# Rust 版另需：rustup stable-x86_64-pc-windows-msvc + Windows SDK；首次构建联网下载
# WinAppSDK redist ZIP（约 66 MB，缓存于 installer/cache/）

powershell -File installer\build.ps1                     # 两版都构建
powershell -File installer\build.ps1 -Version 1.2.0      # 指定版本号
powershell -File installer\build.ps1 -Target dotnet      # 只构建 .NET 版
powershell -File installer\build.ps1 -Target rust        # 只构建 Rust 版
powershell -File installer\build.ps1 -SkipPublish        # 复用已有编译产物，只重打 MSI
```

产物输出到 `installer/output/`，命名规则为 **程序名-操作系统-cpu架构-版本号**
（静默安装：`msiexec /i <包>.msi /qn`；卸载：`msiexec /x <ProductCode> /qn`）。

中间目录（均已 gitignore）：`installer/cache/`（redist ZIP、解出的安装器与 DLL）、
`installer/staging/rust/`（Rust 版 MSI 的收割暂存：`jp-app.exe` + `Microsoft.WindowsAppRuntime.dll`）。

## Rust 版的运行时处理

Rust 版是未打包（unpackaged）运行，不能像 .NET 版那样自带全部运行时：

1. **exe 旁的 `Microsoft.WindowsAppRuntime.dll`**：`winui3` 的 `MsixDynamicDependency` 是静态导入，
   缺这份 DLL 进程在 `main` 之前 `0xC0000135` 退出。build.ps1 从 redist ZIP 的
   Framework MSIX（`MSIX/win10-x64/Microsoft.WindowsAppRuntime.2.msix`）里解出，**与 redist 版本严格一致**。
2. **WinAppSDK 框架包**：运行期 `PackageDependency::initialize()` 挂接目标机上已注册的
   `Microsoft.WindowsAppRuntime.2` 框架包。MSI 把官方 `WindowsAppRuntimeInstall-x64.exe` 存进
   Binary 表（不占安装目录），安装时以 `--quiet` 静默执行——按用户注册
   Framework/Main/Singleton/DDLM 包，无需管理员，已装同版本时幂等跳过。
   卸载**不**移除这些共享运行时包。

升级 WinAppSDK 版本时改 build.ps1 顶部的 `$WinAppSdkVersion` / `$WinAppSdkRedistUrl`
（两个值必须配套），并确认与 `winui3` crate 探测的运行时版本兼容（见 `src-rs/README.md`）。

## 维护注意

- **不要把 Release 的 `SelfContained` / `WindowsAppSDKSelfContained` 改回 false**：
  .NET 版 MSI 不装任何运行时，改回框架依赖会让应用在干净机器上无法启动。
  （Debug 配置保持框架依赖，依赖系统安装的运行时。）
- MSI 的升级由 `MajorUpgrade` 处理：`UpgradeCode` 固定、`ProductCode` 每次构建自动生成，
  装新版本会自动替换旧版本（同 UpgradeCode 内），降级被拦截。
- **两个 wxs 的 `UpgradeCode` 不可互换、不可再生成**：换了 UpgradeCode，老用户的旧安装
  就不会被新版本替换，会留下两个并存的产品。
