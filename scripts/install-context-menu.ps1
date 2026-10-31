$ErrorActionPreference = 'Stop'
$key = 'Registry::HKEY_CURRENT_USER\Software\Classes\*\shell\Zonendpoint'
$commandKey = Join-Path $key 'command'
$exe = 'E:\Zonendpoint\src-tauri\target\release\zonendpoint.exe'
New-Item -Path $commandKey -Force | Out-Null
Set-Item -Path $key -Value 'Scan with Zonendpoint'
Set-Item -Path $commandKey -Value ('"{0}" --scan "%1"' -f $exe)
Write-Host "Installed per-user menu. Build the release executable first: $exe"
