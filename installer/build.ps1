# 构建 Jekyll Post Tool 安装程序（WiX v5 MSI）
# 用法：powershell -File installer\build.ps1 [-Version 1.0.0] [-Target all|dotnet|rust] [-SkipPublish]
# 产物：
#   installer\output\JekyllPostTool-windows-x64-<Version>.msi        （.NET 版，自包含）
#   installer\output\JekyllPostTool(Rust)-windows-x64-<Version>.msi  （Rust 版，带 WinAppSDK redist）
# 依赖：
#   - WiX Toolset 5.x（dotnet tool install -g wix）；UI 扩展缺失时本脚本自动安装
#   - .NET 10 SDK（dotnet 版）
#   - rustup stable-x86_64-pc-windows-msvc + Windows SDK（rust 版，build.rs 编图标资源）
#   - 联网（首次构建 rust 版需下载 WinAppSDK redist ZIP，约 66 MB，缓存于 installer\cache）
param(
    [string]$Version = "1.0.0",
    [ValidateSet("all", "dotnet", "rust")]
    [string]$Target = "all",
    [switch]$SkipPublish
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

# Rust 版依赖的 WinAppSDK 运行时版本（winui3 的 PackageDependency 探测 2.5 起；
# exe 旁的 Microsoft.WindowsAppRuntime.dll 必须与该 redist 同版本，见 src-rs/README.md）
$WinAppSdkVersion = "2.5.1"
$WinAppSdkRedistUrl = "https://aka.ms/windowsappsdk/2.5/$WinAppSdkVersion/Microsoft.WindowsAppRuntime.Redist.2.5.zip"
$WixUiExtVersion = "5.0.2"

function Find-Wix {
    $cmd = Get-Command wix -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    $candidate = "$env:USERPROFILE\.dotnet\tools\wix.exe"
    if (Test-Path $candidate) { return $candidate }
    throw "未找到 wix。请先安装：dotnet tool install -g wix"
}

function Ensure-WixExtensions([string]$wix) {
    $installed = & $wix extension list -g 2>$null | Out-String
    if ($installed -notmatch "WixToolset\.UI\.wixext") {
        Write-Host "安装 WiX UI 扩展 $WixUiExtVersion ..." -ForegroundColor Yellow
        & $wix extension add -g "WixToolset.UI.wixext/$WixUiExtVersion"
        if ($LASTEXITCODE -ne 0) { throw "WiX UI 扩展安装失败" }
    }
}

function Build-Dotnet {
    param([string]$wix)
    $publishDir = "$repoRoot\src\JekyllPostTool.App\bin\Release\Publish"
    if (-not $SkipPublish) {
        Write-Host "== 发布 .NET 应用（Release，自包含）==" -ForegroundColor Cyan
        dotnet publish "$repoRoot\src\JekyllPostTool.App\JekyllPostTool.App.csproj" `
            -c Release -p:PublishProfile=FolderProfile
        if ($LASTEXITCODE -ne 0) { throw "dotnet publish 失败" }
    }
    if (-not (Test-Path "$publishDir\JekyllPostTool.App.exe")) {
        throw "未找到发布产物 $publishDir\JekyllPostTool.App.exe（去掉 -SkipPublish 重新构建？）"
    }

    $out = "$PSScriptRoot\output\JekyllPostTool-windows-x64-$Version.msi"
    Write-Host "== 编译 .NET 版 MSI ==" -ForegroundColor Cyan
    & $wix build "$PSScriptRoot\JekyllPostTool.wxs" `
        -ext WixToolset.UI.wixext -culture zh-CN `
        -d "Version=$Version" -d "PublishDir=$publishDir" `
        -o $out
    if ($LASTEXITCODE -ne 0) { throw "wix build（.NET 版）失败" }
    Write-Host "已生成：$out" -ForegroundColor Green
}

function Get-WinAppSdkRedist {
    # 下载 redist ZIP（缓存复用），解出：
    #   1. WindowsAppRuntimeInstall-x64.exe —— 内嵌进 MSI 的 Binary 表，安装时静默注册运行时包
    #   2. Microsoft.WindowsAppRuntime.dll  —— 从 Framework MSIX 解出，放到 exe 旁（静态导入）
    param()
    $cacheDir = "$PSScriptRoot\cache"
    New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null
    $installerExe = "$cacheDir\WindowsAppRuntimeInstall-x64-$WinAppSdkVersion.exe"
    $bootstrapDll = "$cacheDir\Microsoft.WindowsAppRuntime-$WinAppSdkVersion.dll"
    if ((Test-Path $installerExe) -and (Test-Path $bootstrapDll)) {
        return @{ Installer = $installerExe; Dll = $bootstrapDll }
    }

    $zip = "$cacheDir\winappsdk-redist-$WinAppSdkVersion.zip"
    if (-not (Test-Path $zip)) {
        Write-Host "下载 WinAppSDK $WinAppSdkVersion redist（约 66 MB）..." -ForegroundColor Yellow
        curl.exe -sSL --fail -o $zip $WinAppSdkRedistUrl
        if ($LASTEXITCODE -ne 0) { Remove-Item $zip -ErrorAction SilentlyContinue; throw "redist 下载失败：$WinAppSdkRedistUrl" }
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::OpenRead($zip)
    try {
        $entry = $archive.GetEntry("WindowsAppSDK-Installer-x64/WindowsAppRuntimeInstall-x64.exe")
        if (-not $entry) { throw "redist ZIP 里未找到 WindowsAppRuntimeInstall-x64.exe" }
        [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $installerExe, $true)

        $msixEntry = $archive.GetEntry("MSIX/win10-x64/Microsoft.WindowsAppRuntime.2.msix")
        if (-not $msixEntry) { throw "redist ZIP 里未找到 Microsoft.WindowsAppRuntime.2.msix" }
        $msixTmp = "$cacheDir\framework-x64.msix"
        [System.IO.Compression.ZipFileExtensions]::ExtractToFile($msixEntry, $msixTmp, $true)
        $msix = [System.IO.Compression.ZipFile]::OpenRead($msixTmp)
        try {
            $dllEntry = $msix.GetEntry("Microsoft.WindowsAppRuntime.dll")
            if (-not $dllEntry) { throw "Framework MSIX 里未找到 Microsoft.WindowsAppRuntime.dll" }
            [System.IO.Compression.ZipFileExtensions]::ExtractToFile($dllEntry, $bootstrapDll, $true)
        }
        finally { $msix.Dispose(); Remove-Item $msixTmp -ErrorAction SilentlyContinue }
    }
    finally { $archive.Dispose() }
    return @{ Installer = $installerExe; Dll = $bootstrapDll }
}

function Build-Rust {
    param([string]$wix)
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
    if (-not $cargo) {
        $candidate = "$env:USERPROFILE\.cargo\bin\cargo.exe"
        if (Test-Path $candidate) { $cargo = $candidate }
        else { throw "未找到 cargo。请先安装 rustup（stable-x86_64-pc-windows-msvc）" }
    }

    if (-not $SkipPublish) {
        Write-Host "== 编译 Rust 应用（Release）==" -ForegroundColor Cyan
        Push-Location "$repoRoot\src-rs"
        try {
            & $cargo build --release --bin jp-app
            if ($LASTEXITCODE -ne 0) { throw "cargo build 失败" }
        }
        finally { Pop-Location }
    }
    $appExe = "$repoRoot\src-rs\target\release\jp-app.exe"
    if (-not (Test-Path $appExe)) {
        throw "未找到 $appExe（去掉 -SkipPublish 重新构建？）"
    }

    Write-Host "== 准备 WinAppSDK $WinAppSdkVersion 运行时二进制 ==" -ForegroundColor Cyan
    $redist = Get-WinAppSdkRedist

    # 暂存收割目录：应用 exe + 引导 DLL（版本必须与 redist 一致）
    $stage = "$PSScriptRoot\staging\rust"
    Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $stage | Out-Null
    Copy-Item $appExe "$stage\jp-app.exe"
    Copy-Item $redist.Dll "$stage\Microsoft.WindowsAppRuntime.dll"

    $out = "$PSScriptRoot\output\JekyllPostTool(Rust)-windows-x64-$Version.msi"
    Write-Host "== 编译 Rust 版 MSI ==" -ForegroundColor Cyan
    & $wix build "$PSScriptRoot\JekyllPostTool.Rust.wxs" `
        -ext WixToolset.UI.wixext -culture zh-CN `
        -d "Version=$Version" -d "RustStage=$stage" -d "RuntimeInstaller=$($redist.Installer)" `
        -o $out
    if ($LASTEXITCODE -ne 0) { throw "wix build（Rust 版）失败" }
    Write-Host "已生成：$out" -ForegroundColor Green
}

$wix = Find-Wix
Ensure-WixExtensions $wix
New-Item -ItemType Directory -Force -Path "$PSScriptRoot\output" | Out-Null

if ($Target -in @("all", "dotnet")) { Build-Dotnet $wix }
if ($Target -in @("all", "rust"))    { Build-Rust $wix }
