[Setup]
AppId={{B7D3C6F4-7D9A-4B1C-9A4E-2E5B9D8C6A12}
AppName=API 签到助手
AppVersion=1.0.0
AppPublisher=Minecraft-1314
VersionInfoVersion=1.0.0
DefaultDirName={autopf}\ApiCheckinHelper
DefaultGroupName=API 签到助手
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=ApiCheckinHelper-Setup
SetupIconFile=..\app.ico
UninstallDisplayIcon={app}\app.ico
UninstallDisplayName=API 签到助手
Uninstallable=yes
CreateUninstallRegKey=yes
WizardStyle=modern dynamic windows11
ShowLanguageDialog=yes
Compression=lzma2
SolidCompression=yes
CloseApplications=yes

[Languages]
Name: "zh"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\dist\ApiCheckinHelper-win-x64\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\target\release\uninstall.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\app.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\API 签到助手"; Filename: "{app}\ApiCheckinHelper.exe"; IconFilename: "{app}\app.ico"
Name: "{group}\卸载 API 签到助手"; Filename: "{uninstallexe}"; IconFilename: "{app}\app.ico"
Name: "{autodesktop}\API 签到助手"; Filename: "{app}\ApiCheckinHelper.exe"; IconFilename: "{app}\app.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\ApiCheckinHelper.exe"; Description: "{cm:LaunchProgram,API 签到助手}"; Flags: nowait postinstall skipifsilent
