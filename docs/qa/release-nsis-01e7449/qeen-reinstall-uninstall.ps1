$ErrorActionPreference='Stop'
$base='E:\Zonendpoint-tools\installer-sandbox'
$install=[IO.Path]::GetFullPath((Join-Path $base 'app'))
if($install -ne 'E:\Zonendpoint-tools\installer-sandbox\app'){throw 'Unexpected target'}
if((Get-Item -LiteralPath $install).Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'Reparse target refused'}
$exe=Join-Path $install 'zonendpoint.exe'
$hash=(Get-FileHash -LiteralPath $exe).Hash
$cache='C:\Users\admzon\AppData\Local\Zonendpoint\cache.db'
$original='2DE45CDEC359752C767752891E45B95E0B9BDB2ABCBCA75D63EF3576F331EA5E'
if((Get-FileHash -LiteralPath $cache).Hash -ne $original){throw 'Original cache baseline changed'}
$sentinel=Join-Path $base 'userdata\preserve-me.txt'
if(-not(Test-Path -LiteralPath $sentinel)){throw 'Missing private data sentinel'}
Remove-Item -LiteralPath $exe
$setup='E:\Zonendpoint-tools\qeen-target\release\bundle\nsis\Zonendpoint_0.1.0_x64-setup.exe'
$p=Start-Process $setup -ArgumentList @('/S',"/D=$install") -WindowStyle Hidden -Wait -PassThru
if($p.ExitCode -ne 0 -or -not(Test-Path -LiteralPath $exe)){throw 'Repair install failed'}
if((Get-FileHash -LiteralPath $exe).Hash -ne $hash){throw 'Repair payload changed'}
@{ReinstallExit=$p.ExitCode;PayloadSHA256=$hash;OriginalCacheSHA256=$original;Sentinel=$sentinel} | ConvertTo-Json | Set-Content (Join-Path $base 'reinstall-result.json')
$registration=@(Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue | Where-Object {$_.DisplayName -match '^Zonendpoint'})
if($registration.Count -ne 1 -or $registration[0].UninstallString -notlike '*installer-sandbox\app\uninstall.exe*'){throw 'Uninstall identity mismatch'}
$p=Start-Process -FilePath (Join-Path $install 'uninstall.exe') -ArgumentList '/S' -WindowStyle Hidden -Wait -PassThru
for($i=0;$i -lt 100 -and (Test-Path -LiteralPath $exe);$i++){Start-Sleep -Milliseconds 100}
if(Test-Path -LiteralPath $exe){throw 'Executable remains'}
$remaining=@(Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue | Where-Object {$_.DisplayName -match '^Zonendpoint'})
if($remaining.Count){throw 'Registry remains'}
foreach($l in (Get-Content (Join-Path $base 'shortcuts.json') -Raw | ConvertFrom-Json)){if(Test-Path -LiteralPath $l.Path){throw 'Shortcut remains'}}
if(-not(Test-Path -LiteralPath $sentinel)){throw 'Private data lost'}
if((Get-FileHash -LiteralPath $cache).Hash -ne $original){throw 'Original cache changed'}
@{UninstallExit=$p.ExitCode;ExecutableRemoved=$true;RegistryRemoved=$true;ShortcutsRemoved=$true;PrivateDataPreserved=$true;OriginalCacheUnchanged=$true} | ConvertTo-Json | Set-Content (Join-Path $base 'uninstall-result.json')
Write-Output 'FINAL REINSTALL/UNINSTALL PASS; DATA PRESERVED'
