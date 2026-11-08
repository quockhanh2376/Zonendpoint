<#
.SYNOPSIS
    Installs the ZonEndpoint persistent SSH tunnel Scheduled Task for the current user.
.DESCRIPTION
    Creates a user-level Scheduled Task that runs zon-tunnel.ps1 at user logon in the background
    without requiring administrator privileges.
#>

$ErrorActionPreference = 'Stop'

$taskName = "ZonEndpointTunnel"
$targetDir = Join-Path $env:LOCALAPPDATA "ZonEndpoint"
if (-not (Test-Path $targetDir)) {
    New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
}

$sourceScript = Join-Path $PSScriptRoot "zon-tunnel.ps1"
$installedScript = Join-Path $targetDir "zon-tunnel.ps1"

# Copy supervisor script to user appdata for permanence across worktree changes
Copy-Item -Path $sourceScript -Destination $installedScript -Force
Write-Host "Installed tunnel script to: $installedScript"

# Unregister existing task if present
$existing = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
if ($existing) {
    Write-Host "Removing existing Scheduled Task '$taskName'..."
    Stop-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false
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
    -Priority 7

Register-ScheduledTask `
    -TaskName $taskName `
    -Action $action `
    -Trigger $trigger `
    -Settings $settings `
    -Description "ZonEndpoint persistent ClamAV SSH tunnel" `
    -Force | Out-Null

Write-Host "Registered Scheduled Task '$taskName' for user $env:USERNAME (At logon, Hidden, non-admin)."

# Start task immediately
Start-ScheduledTask -TaskName $taskName
Write-Host "Started Scheduled Task '$taskName'."
