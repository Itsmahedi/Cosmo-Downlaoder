; RustGet Inno Setup Script
; High-Performance Multi-Segment Download Manager for Windows 11

#define MyAppName "RustGet"
#define MyAppVersion "2.0.0"
#define MyAppPublisher "RustGet Contributors"
#define MyAppURL "https://github.com/rustget/rustget"
#define MyAppExeName "rustget-gui.exe"
#define MyCliExeName "rustget.exe"
#define MyHostExeName "rustget-host.exe"

[Setup]
AppId={{9B78B45B-764E-40DC-8C37-A77E2EF5D2EA}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=..\LICENSE
OutputDir=dist
OutputBaseFilename=RustGet-Setup-{#MyAppVersion}
SetupIconFile=icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startupicon"; Description: "Start RustGet automatically with Windows"; GroupDescription: "Windows Integration:"
Name: "registerbrowsers"; Description: "Register browser extension bridge (Chrome & Edge)"; GroupDescription: "Browser Integration:"

[Files]
; Binaries compiled in release mode
Source: "..\target\x86_64-pc-windows-gnullvm\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\x86_64-pc-windows-gnullvm\release\{#MyCliExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\x86_64-pc-windows-gnullvm\release\{#MyHostExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\yt-dlp.exe"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist

; App Icon
Source: "icon.ico"; DestDir: "{app}"; Flags: ignoreversion

; Native Messaging Manifests
Source: "..\extension\com.rustget.downloader.chrome.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\extension\com.rustget.downloader.edge.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\extension\com.rustget.downloader.firefox.json"; DestDir: "{app}"; Flags: ignoreversion

; Extension folder for Chrome/Edge & Firefox
Source: "..\extension\*"; DestDir: "{app}\extension"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\extension-firefox\*"; DestDir: "{app}\extension-firefox"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\icon.ico"
Name: "{group}\RustGet Command Line"; Filename: "{app}\{#MyCliExeName}"; IconFilename: "{app}\icon.ico"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\icon.ico"; Tasks: desktopicon

[Registry]
; Auto-start with Windows (optional task)
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "{#MyAppName}"; ValueData: """{app}\{#MyAppExeName}"" --minimized"; Tasks: startupicon; Flags: uninsdeletevalue

; Chrome Native Messaging Host Registration
Root: HKCU; Subkey: "Software\Google\Chrome\NativeMessagingHosts\com.rustget.downloader"; ValueType: string; ValueData: "{app}\com.rustget.downloader.chrome.json"; Tasks: registerbrowsers; Flags: uninsdeletekey

; Edge Native Messaging Host Registration
Root: HKCU; Subkey: "Software\Microsoft\Edge\NativeMessagingHosts\com.rustget.downloader"; ValueType: string; ValueData: "{app}\com.rustget.downloader.edge.json"; Tasks: registerbrowsers; Flags: uninsdeletekey

; Firefox Native Messaging Host Registration
Root: HKCU; Subkey: "Software\Mozilla\NativeMessagingHosts\com.rustget.downloader"; ValueType: string; ValueData: "{app}\com.rustget.downloader.firefox.json"; Tasks: registerbrowsers; Flags: uninsdeletekey

; Add to User PATH so 'rustget' can be called from any terminal
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
Filename: "{app}\{#MyHostExeName}"; Parameters: "--register ""{app}\com.rustget.downloader.chrome.json"""; Flags: runhidden; Tasks: registerbrowsers
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{app}\{#MyHostExeName}"; Parameters: "--unregister"; RunOnceId: "RustGetUninstallHost"; Flags: runhidden

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + UpperCase(Param) + ';', ';' + UpperCase(OrigPath) + ';') = 0;
end;
