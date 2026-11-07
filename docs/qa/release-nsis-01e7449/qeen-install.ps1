param([Parameter(Mandatory=$true)][string]$Installer)
$ErrorActionPreference='Stop'
$base='E:\Zonendpoint-tools\installer-sandbox'
$target=Join-Path $base 'app'
if(Test-Path -LiteralPath $target){throw 'Test installation directory already exists; inspect before continuing'}
$old=@(Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue | Where-Object {$_.DisplayName -match '^Zonendpoint'})
if($old.Count){throw 'Existing Zonendpoint installation found; preserve it'}
New-Item -ItemType Directory -Force $base | Out-Null
$before=Get-FileHash -LiteralPath $Installer -Algorithm SHA256
$process=Start-Process -FilePath $Installer -ArgumentList @('/S',"/D=$target") -WindowStyle Hidden -PassThru -Wait
if($process.ExitCode -ne 0){throw "Installer returned $($process.ExitCode)"}
$exe=Join-Path $target 'zonendpoint.exe'
if(-not(Test-Path -LiteralPath $exe)){throw 'Installed executable missing'}
$registration=@(Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue | Where-Object {$_.DisplayName -match '^Zonendpoint'})
if($registration.Count -ne 1){throw 'Expected one per-user uninstall registration'}
$registration | Select-Object DisplayName,DisplayVersion,InstallLocation,UninstallString | ConvertTo-Json | Set-Content (Join-Path $base 'registration.json')
$shortcutRoots=@([Environment]::GetFolderPath('Programs'),[Environment]::GetFolderPath('Desktop'))
$links=@(foreach($r in $shortcutRoots){Get-ChildItem -LiteralPath $r -Filter '*Zonendpoint*.lnk' -Recurse -ErrorAction SilentlyContinue})
if(-not $links.Count){throw 'No installed shortcut found'}
$shell=New-Object -ComObject WScript.Shell
$shortcuts=@(foreach($l in $links){$s=$shell.CreateShortcut($l.FullName);[pscustomobject]@{Path=$l.FullName;Target=$s.TargetPath;Icon=$s.IconLocation}})
if(-not @($shortcuts | Where-Object {$_.Target -eq $exe}).Count){throw 'No shortcut points to test executable'}
$shortcuts | ConvertTo-Json | Set-Content (Join-Path $base 'shortcuts.json')
[pscustomobject]@{Installer=$Installer;SHA256=$before.Hash;InstalledExe=$exe;ExitCode=$process.ExitCode;Version=(Get-Item $exe).VersionInfo.FileVersion;Signature=(Get-AuthenticodeSignature -LiteralPath $exe).Status} | ConvertTo-Json | Set-Content (Join-Path $base 'install-result.json')
Write-Output "INSTALL PASS $exe"
