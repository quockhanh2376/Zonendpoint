<#
.SYNOPSIS
    Uninstalls the ZonEndpoint persistent SSH tunnel Scheduled Task.
.DESCRIPTION
    Stops and removes the Scheduled Task and terminates running tunnel processes.
#>

$ErrorActionPreference = 'Continue'

$taskName = "ZonEndpointTunnel"

# Stop and unregister scheduled task
$existing = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
if ($existing) {
    Write-Host "Stopping and unregistering Scheduled Task '$taskName'..."
    Stop-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
    Write-Host "Unregistered Scheduled Task '$taskName'."
} else {
    Write-Host "Scheduled Task '$taskName' not found."
}

# Stop any running supervisor powershell processes
$supervisors = Get-CimInstance Win32_Process -Filter "Name = 'powershell.exe'" |
    Where-Object { $_.CommandLine -like "*zon-tunnel.ps1*" }

foreach ($p in $supervisors) {
    Write-Host "Stopping tunnel supervisor process PID $($p.ProcessId)..."
    Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
}

# Stop any running ssh processes forwarding port 13310
$sshProcesses = Get-CimInstance Win32_Process -Filter "Name = 'ssh.exe'" |
    Where-Object { $_.CommandLine -like "*13310:127.0.0.1:3310*" -or $_.CommandLine -like "*13310*" }

foreach ($s in $sshProcesses) {
    Write-Host "Stopping ssh tunnel process PID $($s.ProcessId)..."
    Stop-Process -Id $s.ProcessId -Force -ErrorAction SilentlyContinue
}

Write-Host "ZonEndpoint tunnel uninstall completed."
