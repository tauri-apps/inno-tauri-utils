; Copyright 2026 inno-tauri-utils contributors
; SPDX-License-Identifier: Apache-2.0 OR MIT
; Run by the Rust integration test with matching DLL and installer architectures.

#if Ver < EncodeVer(6, 4, 0)
  #error Inno Setup 6.4 or later is required
#endif
#ifndef InstallerArchitecture
  #define InstallerArchitecture "x86"
#endif
#if (InstallerArchitecture != "x86") && (InstallerArchitecture != "x64")
  #error InstallerArchitecture must be x86 or x64
#endif
#if (InstallerArchitecture == "x64") && (Ver < EncodeVer(7, 0, 0))
  #error x64 installers require Inno Setup 7 or later
#endif

[Setup]
AppName=Inno utilities integration tests
AppVersion=1.0.0
DefaultDirName={tmp}\inno-utils-tests
PrivilegesRequired=lowest
CreateAppDir=no
Uninstallable=no
DisableDirPage=yes
DisableProgramGroupPage=yes
OutputDir=..\target\inno-tests
OutputBaseFilename=smoke
#if Ver >= EncodeVer(7, 0, 0)
SetupArchitecture={#InstallerArchitecture}
#endif

[Files]
Source: "{#DllDir}\inno_semvercompare.dll"; Flags: dontcopy noencryption

[Code]
var Report: String;

function SemverCompare(LeftVersion, RightVersion: String): Integer;
  external 'SemverCompare@files:inno_semvercompare.dll stdcall delayload setuponly';

procedure Check(Expected: Integer; const LeftVersion, RightVersion, Description: String);
var Actual: Integer;
begin
  Actual := SemverCompare(LeftVersion, RightVersion);
  if Actual <> Expected then
    RaiseException(Format('%s: expected %d, got %d', [Description, Expected, Actual]));
  Report := Report + 'PASS ' + Description + #13#10;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var I: Integer; ReportPath: String;
begin
  Result := '';
  Report := '';
  ReportPath := ExpandConstant('{param:RESULTFILE|}');
  if ReportPath = '' then begin
    Result := 'The test requires /RESULTFILE=<path>.';
    Exit;
  end;
  try
    Check(-1, '1.2.0', '1.2.1', 'older version');
    Check(1, '2.0.0', '1.9.9', 'newer version');
    Check(0, '1.2.3', '1.2.3', 'equal versions');
    Check(-1, '1.2.3-rc.1', '1.2.3', 'prerelease before release');
    Check(-1, '1.2.3-beta.2', '1.2.3-beta.11', 'numeric prerelease ordering');
    Check(0, '1.2.3+z', '1.2.3+a', 'build metadata ignored');
    Check(2, 'invalid', '1.2.3', 'invalid left version');
    Check(3, '1.2.3', 'invalid', 'invalid right version');
    Check(4, 'invalid', 'invalid', 'both versions invalid');
    Check(2, '', '1.2.3', 'empty left version');
    Check(3, '1.2.3', '', 'empty right version');
    Check(2, '1.2.3-' + #233, '1.2.3', 'non-ASCII version rejected');
    Check(2, '1.2.3-' + #55357, '1.2.3', 'unpaired UTF-16 surrogate rejected');
    Check(0, '1.2.3' + #0 + 'ignored', '1.2.3', 'null terminator respected');
    Check(0, '1.2.3+' + StringOfChar('a', 1018), '1.2.3', '1024-unit version accepted');
    Check(2, '1.2.3+' + StringOfChar('a', 1019), '1.2.3', 'overlong left version');
    Check(3, '1.2.3', '1.2.3+' + StringOfChar('a', 1019), 'overlong right version');
    for I := 1 to 1000 do
      if SemverCompare('1.0.0', '1.1.0') <> -1 then
        RaiseException('Repeated DLL call failed.');
    Report := Report + 'PASS repeated DLL calls' + #13#10 + 'PASS' + #13#10;
  except
    Result := 'FAIL ' + GetExceptionMessage;
    Report := Report + Result + #13#10;
  end;
  if not SaveStringToFile(ReportPath, Report, False) then
    Result := 'Could not write the test report.';
end;
