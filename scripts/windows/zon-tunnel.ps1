<#
.SYNOPSIS
    ZonEndpoint persistent SSH tunnel supervisor for Windows.
.DESCRIPTION
    Monitors and automatically restarts the SSH tunnel to the ClamAV lab server.
    Logs events with rotation to %LOCALAPPDATA%\ZonEndpoint\tunnel\tunnel.log (no secrets logged).
#>

$ErrorActionPreference = 'Continue'

$tunnelDir = Join-Path $env:LOCALAPPDATA "ZonEndpoint\tunnel"
if (-not (Test-Path $tunnelDir)) {
    New-Item -ItemType Directory -Path $tunnelDir -Force | Out-Null
}

$logFile = Join-Path $tunnelDir "tunnel.log"
$logBackup = Join-Path $tunnelDir "tunnel.log.1"
$maxLogBytes = 256 * 1024 # 256 KB

function Write-TunnelLog {
    param([string]$Message)
    $timestamp = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
    $line = "[$timestamp] $Message"
    try {
        if ((Test-Path $logFile) -and ((Get-Item $logFile).Length -gt $maxLogBytes)) {
            Move-Item -Path $logFile -Destination $logBackup -Force -ErrorAction SilentlyContinue
        }
        Add-Content -Path $logFile -Value $line -Encoding utf8
    } catch {
        [Console]::Error.WriteLine($line)
    }
}

$keyPath = Join-Path $env:USERPROFILE ".ssh\zonendpoint_lab"
$remoteHost = "admzon@100.96.250.245"
$localPort = 13310
$remotePort = 3310

Write-TunnelLog "=== ZonEndpoint tunnel supervisor starting (PID: $PID) ==="

if (-not (Test-Path $keyPath)) {
    Write-TunnelLog "ERROR: SSH private key not found at $keyPath. Exiting."
    exit 1
}

$backoff = 5
$maxBackoff = 60

while ($true) {
    Write-TunnelLog "Connecting SSH tunnel (127.0.0.1:$localPort -> 127.0.0.1:$remotePort via $remoteHost)..."
    $start = Get-Date

    try {
        & ssh -i $keyPath `
            -o BatchMode=yes `
            -o ExitOnForwardFailure=yes `
            -o ServerAliveInterval=15 `
            -o ServerAliveCountMax=3 `
            -N `
            -L "127.0.0.1:${localPort}:127.0.0.1:${remotePort}" `
            $remoteHost
        $exitCode = $LASTEXITCODE
    } catch {
        $exitCode = -1
        Write-TunnelLog "SSH execution failed: $($_.Exception.Message)"
    }

    $duration = ((Get-Date) - $start).TotalSeconds
    Write-TunnelLog ("SSH exited with code {0} after {1:N1}s." -f $exitCode, $duration)

    if ($duration -ge 30) {
        $backoff = 5
    }

    Write-TunnelLog "Reconnecting in ${backoff}s..."
    Start-Sleep -Seconds $backoff

    $backoff = [Math]::Min($backoff * 2, $maxBackoff)
}
