<#
.SYNOPSIS
    Installs the ZonEndpoint persistent SSH tunnel Scheduled Task for the current user.
.DESCRIPTION
    Creates a user-level Scheduled Task that runs zon-tunnel.ps1 at user logon in the background
    without requiring administrator privileges.
    Stores zon-tunnel.ps1, tunnel-config.json, and tunnel.log in %LOCALAPPDATA%\ZonEndpoint\tunnel\.
#>

$ErrorActionPreference = 'Stop'

$taskName = "ZonEndpointTunnel"
$rootDir = Join-Path $env:LOCALAPPDATA "ZonEndpoint"
$targetDir = Join-Path $rootDir "tunnel"
if (-not (Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
}

$sourceScript = Join-Path $PSScriptRoot "zon-tunnel.ps1"
$installedScript = Join-Path $targetDir "zon-tunnel.ps1"
$sourceExample = Join-Path $PSScriptRoot "tunnel-config.example.json"
$targetConfig = Join-Path $targetDir "tunnel-config.json"
$targetExample = Join-Path $targetDir "tunnel-config.example.json"

# Clean up legacy files in root appdata directory (%LOCALAPPDATA%\ZonEndpoint\)
$legacyFiles = @("zon-tunnel.ps1", "tunnel.log", "tunnel.log.1")
foreach ($f in $legacyFiles) {
    $oldPath = Join-Path $rootDir $f
    if (Test-Path $oldPath) {
        $destPath = Join-Path $targetDir $f
        if ($f -like "tunnel.log*" -and (-not (Test-Path $destPath))) {
            Move-Item -Path $oldPath -Destination $destPath -Force -ErrorAction SilentlyContinue
            Write-Host "Migrated legacy log $f to $targetDir"
        } else {
            Remove-Item -Path $oldPath -Force -ErrorAction SilentlyContinue
            Write-Host "Removed legacy file: $oldPath"
        }
    }
}

# Copy supervisor script to user appdata tunnel directory for permanence across worktree changes
Copy-Item -Path $sourceScript -Destination $installedScript -Force
Write-Host "Installed tunnel script to: $installedScript"

# Copy example config template if present
if (Test-Path $sourceExample) {
    Copy-Item -Path $sourceExample -Destination $targetExample -Force
    Write-Host "Copied configuration template to: $targetExample"
}

# If target config doesn't exist, initialize from example (do not overwrite existing config)
if ((-not (Test-Path $targetConfig)) -and (Test-Path $sourceExample)) {
    Copy-Item -Path $sourceExample -Destination $targetConfig
    Write-Host "Created initial configuration at: $targetConfig"
    Write-Host "NOTICE: Please configure '$targetConfig' before running the tunnel."
}

# Unregister existing task and stop running supervisor if present
$existing = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
if ($existing) {
    Write-Host "Removing existing Scheduled Task '$taskName'..."
    Stop-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
}

$supervisors = Get-CimInstance Win32_Process -Filter "Name = 'powershell.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.CommandLine -like "*zon-tunnel.ps1*" }
foreach ($p in $supervisors) {
    Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
}

$action = New-ScheduledTaskAction `
    -Execute "powershell.exe" `
    -Argument ('-WindowStyle Hidden -ExecutionPolicy Bypass -NoProfile -File "{0}"' -f $installedScript)

$trigger = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME

$settings = New-ScheduledTaskSettingsSet `
    -Hidden `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -ExecutionTimeLimit 0 `
    -Priority 7 `
    -RestartCount 3 `
    -RestartInterval (New-TimeSpan -Minutes 1)

Register-ScheduledTask `
    -TaskName $taskName `
    -Action $action `
    -Trigger $trigger `
    -Settings $settings `
    -Description "ZonEndpoint persistent ClamAV SSH tunnel" `
    -Force | Out-Null

Write-Host "Registered Scheduled Task '$taskName' for user $env:USERNAME (At logon, Hidden, non-admin, RestartCount 3, RestartInterval 1m)."

# Start task immediately
Start-ScheduledTask -TaskName $taskName
Write-Host "Started Scheduled Task '$taskName'."
