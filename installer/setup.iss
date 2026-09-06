; Jekyll Post Tool 安装包脚本（Inno Setup 6.3+）
; 构建：运行 installer/build.ps1，或在发布产物就绪后直接运行 ISCC 编译本脚本。
;
; 发布为完全自包含（.NET 与 WinUI 运行时随应用分发，见 csproj 的 SelfContained /
; WindowsAppSDKSelfContained），因此本安装器：
;   - 不需要检测、下载或安装任何运行时依赖
;   - 按用户安装到 %LOCALAPPDATA%\Programs，无需管理员权限、全程无 UAC

#ifndef AppVersion
#define AppVersion "1.0.0"
#endif

#define MyAppName "Jekyll Post Tool"
#define MyAppExeName "JekyllPostTool.App.exe"

[Setup]
AppId={{3DCE5337-E7F7-4950-B2C1-8FFC91ACA8D7}
AppName={#MyAppName}
AppVersion={#AppVersion}
AppPublisher=self-exiler
DefaultDirName={userpf}\JekyllPostTool
DisableProgramGroupPage=yes
OutputDir=output
; 安装包命名：程序名-操作系统-cpu架构-版本号（架构与 ArchitecturesAllowed=x64compatible 对应）
OutputBaseFilename=JekyllPostTool-windows-x64-{#AppVersion}
SetupIconFile=..\src\JekyllPostTool.App\Assets\AppIcon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
MinVersion=10.0.17763
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
CloseApplications=yes
SetupLogging=yes

[Languages]
Name: "chinese"; MessagesFile: "ChineseSimplified.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\src\JekyllPostTool.App\bin\Release\Publish\*"; Excludes: "*.pdb"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent
