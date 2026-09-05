# 构建 Jekyll Post Tool 安装程序
# 用法：powershell -File installer\build.ps1 [-Version 1.0.0]
# 产物：installer\output\JekyllPostToolSetup-<Version>.exe
# 依赖：Inno Setup 6.7+（winget install JRSoftware.InnoSetup）；ISCC 不在默认路径时会提示。
param(
    [string]$Version = "1.0.0",
    [switch]$SkipPublish
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

if (-not $SkipPublish) {
    Write-Host "== 1/2 发布应用（Release，框架依赖）==" -ForegroundColor Cyan
    dotnet publish "$repoRoot\src\JekyllPostTool.App\JekyllPostTool.App.csproj" `
        -c Release -p:PublishProfile=FolderProfile
    if ($LASTEXITCODE -ne 0) { throw "dotnet publish 失败" }
}
else {
    Write-Host "== 1/2 跳过发布（-SkipPublish）==" -ForegroundColor Yellow
}

Write-Host "== 2/2 编译安装包 ==" -ForegroundColor Cyan
$isccCandidates = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
)
$iscc = $isccCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) {
    throw "未找到 ISCC.exe。请先安装 Inno Setup 6：winget install JRSoftware.InnoSetup"
}

& $iscc "/DAppVersion=$Version" "$PSScriptRoot\setup.iss"
if ($LASTEXITCODE -ne 0) { throw "ISCC 编译失败" }

Write-Host "安装包已生成：$PSScriptRoot\output\JekyllPostToolSetup-$Version.exe" -ForegroundColor Green
