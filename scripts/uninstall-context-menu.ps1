$ErrorActionPreference = 'Stop'
$key = 'Registry::HKEY_CURRENT_USER\Software\Classes\*\shell\Zonendpoint'
if (Test-Path $key) { Remove-Item -LiteralPath $key -Recurse -Force }
Write-Host 'Removed the Zonendpoint context menu.'
