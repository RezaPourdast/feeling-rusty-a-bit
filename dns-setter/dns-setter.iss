[Setup]
AppName=DNS Setter
AppVersion=1.0.0
AppPublisher=Your Name
DefaultDirName={autopf}\DNS Setter
DefaultGroupName=DNS Setter
DisableProgramGroupPage=yes
OutputDir=installer
OutputBaseFilename=DNS-Setter-Setup
Compression=lzma
SolidCompression=yes
WizardStyle=modern
Uninstallable=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64
MinVersion=6.1sp1

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop icon"; GroupDescription: "Additional icons:"; Flags: unchecked
Name: "startmenuicon"; Description: "Create a &Start Menu icon"; GroupDescription: "Additional icons:"

[Files]
Source: "C:\Projects\feeling rusty a bit\dns-setter\target\release\dns-setter.exe"; DestDir: "{app}"; Flags: ignoreversion; DestName: "DNSSetter.exe"
Source: "C:\Projects\feeling rusty a bit\dns-setter\asset\cat.webp"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "C:\Projects\feeling rusty a bit\dns-setter\asset\ferris.gif"; DestDir: "{app}\assets"; Flags: ignoreversion
Source: "C:\Projects\feeling rusty a bit\dns-setter\asset\ferris.svg"; DestDir: "{app}\assets"; Flags: ignoreversion

[Icons]
Name: "{group}\DNS Setter"; Filename: "{app}\DNSSetter.exe"; Tasks: startmenuicon
Name: "{userdesktop}\DNS Setter"; Filename: "{app}\DNSSetter.exe"; Tasks: desktopicon
Name: "{group}\Uninstall DNS Setter"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\DNSSetter.exe"; Description: "Launch DNS Setter"; Flags: nowait postinstall skipifsilent
