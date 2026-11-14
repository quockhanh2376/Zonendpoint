<#
.SYNOPSIS
    ZonEndpoint persistent SSH tunnel supervisor for Windows with signature status sync.
.DESCRIPTION
    Monitors and automatically restarts the SSH tunnel to the ClamAV lab server.
    Periodically synchronizes /var/lib/zonendpoint/status/signatures.json via SSH
    to %LOCALAPPDATA%\ZonEndpoint\server-status.json every ~15 minutes without
    interrupting the persistent tunnel connection.
    Logs events with rotation to %LOCALAPPDATA%\ZonEndpoint\tunnel\tunnel.log (no secrets logged).
#>

param(
    [switch]$SyncOnce
)

$ErrorActionPreference = 'Continue'

$appDataDir = Join-Path $env:LOCALAPPDATA "ZonEndpoint"
$tunnelDir = Join-Path $appDataDir "tunnel"
if (-not (Test-Path $tunnelDir)) {
    New-Item -ItemType Directory -Path $tunnelDir -Force | Out-Null
}

$statusFile = Join-Path $appDataDir "server-status.json"
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
$statusSyncIntervalSecs = 900 # 15 minutes

function Fetch-ServerStatus {
    param(
        [string]$Key,
        [string]$HostTarget,
        [string]$TargetFile,
        [string]$LogPath
    )

    $writeLog = {
        param([string]$msg)
        $ts = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
        $entry = "[$ts] [StatusSync] $msg"
        try { Add-Content -Path $LogPath -Value $entry -Encoding utf8 } catch {}
    }

    try {
        if (-not (Test-Path $Key)) {
            & $writeLog "SSH key file missing, cannot fetch status."
            return $false
        }

        $sshOutput = & ssh -i $Key `
            -o BatchMode=yes `
            -o ConnectTimeout=10 `
            -o ServerAliveInterval=5 `
            -o ServerAliveCountMax=2 `
            $HostTarget `
            "cat /var/lib/zonendpoint/status/signatures.json" 2>$null

        if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($sshOutput)) {
            & $writeLog "Failed to retrieve status file from server (exit code $LASTEXITCODE)."
            return $false
        }

        $rawText = ($sshOutput -join "`n").Trim()
        $json = $rawText | ConvertFrom-Json -ErrorAction Stop

        if ($null -eq $json.signatures) {
            & $writeLog "Validation failed: 'signatures' field is missing."
            return $false
        }

        $sigVal = $json.signatures
        if (-not ($sigVal -is [int] -or $sigVal -is [long] -or $sigVal -is [double])) {
            & $writeLog "Validation failed: 'signatures' is not a number."
            return $false
        }

        $sigLong = [long]$sigVal
        if ($sigLong -lt 0 -or $sigLong -gt 1000000000) {
            & $writeLog "Validation failed: 'signatures' value $sigLong outside valid range 0..1000000000."
            return $false
        }

        if ([string]::IsNullOrWhiteSpace($json.loaded_at_utc)) {
            & $writeLog "Validation failed: 'loaded_at_utc' missing or empty."
            return $false
        }

        $parsedDate = [DateTime]::MinValue
        if (-not [DateTime]::TryParse($json.loaded_at_utc, [ref]$parsedDate)) {
            & $writeLog "Validation failed: 'loaded_at_utc' ($($json.loaded_at_utc)) is not a valid datetime string."
            return $false
        }

        $targetDir = Split-Path $TargetFile -Parent
        if (-not (Test-Path $targetDir)) {
            New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
        }

        $tempFile = "$TargetFile.tmp.$PID"
        [System.IO.File]::WriteAllText($tempFile, $rawText, [System.Text.Encoding]::UTF8)
        Move-Item -Path $tempFile -Destination $TargetFile -Force -ErrorAction Stop
        & $writeLog "Successfully updated server status: signatures=$sigLong, loaded_at_utc=$($json.loaded_at_utc)"
        return $true
    } catch {
        & $writeLog "Exception during server status sync: $($_.Exception.Message)"
        return $false
    }
}

if ($SyncOnce) {
    Write-TunnelLog "=== ZonEndpoint manual status fetch invoked ==="
    $ok = Fetch-ServerStatus -Key $keyPath -HostTarget $remoteHost -TargetFile $statusFile -LogPath $logFile
    if ($ok) {
        Write-Output "Status fetch succeeded. Target: $statusFile"
        Get-Content $statusFile
    } else {
        Write-Error "Status fetch failed. Check $logFile."
    }
    return
}

Write-TunnelLog "=== ZonEndpoint tunnel supervisor starting (PID: $PID) ==="

if (-not (Test-Path $keyPath)) {
    Write-TunnelLog "ERROR: SSH private key not found at $keyPath. Exiting."
    exit 1
}

# Start background runspace for periodic status sync so ssh -N does not block sync
$runspace = [runspacefactory]::CreateRunspace()
$runspace.Open()
$statusPs = [powershell]::Create()
$statusPs.Runspace = $runspace

$syncScript = {
    param($keyPath, $remoteHost, $statusFile, $logFile, $intervalSecs)

    function Sync-Loop {
        param($Key, $HostTarget, $TargetFile, $LogPath)
        $writeLog = {
            param([string]$msg)
            $ts = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
            $entry = "[$ts] [StatusSync] $msg"
            try { Add-Content -Path $LogPath -Value $entry -Encoding utf8 } catch {}
        }

        try {
            if (-not (Test-Path $Key)) { return }
            $sshOutput = & ssh -i $Key `
                -o BatchMode=yes `
                -o ConnectTimeout=10 `
                -o ServerAliveInterval=5 `
                -o ServerAliveCountMax=2 `
                $HostTarget `
                "cat /var/lib/zonendpoint/status/signatures.json" 2>$null

            if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($sshOutput)) {
                & $writeLog "Failed to retrieve status file from server (exit code $LASTEXITCODE)."
                return
            }

            $rawText = ($sshOutput -join "`n").Trim()
            $json = $rawText | ConvertFrom-Json -ErrorAction Stop
            if ($null -eq $json.signatures) { return }
            $sigVal = $json.signatures
            if (-not ($sigVal -is [int] -or $sigVal -is [long] -or $sigVal -is [double])) { return }
            $sigLong = [long]$sigVal
            if ($sigLong -lt 0 -or $sigLong -gt 1000000000) { return }
            if ([string]::IsNullOrWhiteSpace($json.loaded_at_utc)) { return }
            $parsedDate = [DateTime]::MinValue
            if (-not [DateTime]::TryParse($json.loaded_at_utc, [ref]$parsedDate)) { return }

            $targetDir = Split-Path $TargetFile -Parent
            if (-not (Test-Path $targetDir)) {
                New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
            }
            $tempFile = "$TargetFile.tmp.$PID"
            [System.IO.File]::WriteAllText($tempFile, $rawText, [System.Text.Encoding]::UTF8)
            Move-Item -Path $tempFile -Destination $TargetFile -Force -ErrorAction Stop
            & $writeLog "Successfully updated server status: signatures=$sigLong, loaded_at_utc=$($json.loaded_at_utc)"
        } catch {
            & $writeLog "Exception during server status sync: $($_.Exception.Message)"
        }
    }

    # Initial sync on launch
    Sync-Loop -Key $keyPath -HostTarget $remoteHost -TargetFile $statusFile -LogPath $logFile

    while ($true) {
        Start-Sleep -Seconds $intervalSecs
        Sync-Loop -Key $keyPath -HostTarget $remoteHost -TargetFile $statusFile -LogPath $logFile
    }
}

[void]$statusPs.AddScript($syncScript).AddArgument($keyPath).AddArgument($remoteHost).AddArgument($statusFile).AddArgument($logFile).AddArgument($statusSyncIntervalSecs)
$statusAsyncHandle = $statusPs.BeginInvoke()
Write-TunnelLog "Started background server-status sync runspace (interval: ${statusSyncIntervalSecs}s)."

$backoff = 5
$maxBackoff = 60

try {
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
} finally {
    if ($statusPs) {
        $statusPs.Stop()
        $statusPs.Dispose()
    }
    if ($runspace) {
        $runspace.Close()
        $runspace.Dispose()
    }
}
