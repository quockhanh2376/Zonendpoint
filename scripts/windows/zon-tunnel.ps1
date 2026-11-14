<#
.SYNOPSIS
    ZonEndpoint persistent SSH tunnel supervisor for Windows with signature status sync.
.DESCRIPTION
    Monitors and automatically restarts the SSH tunnel to the ClamAV lab server.
    Periodically synchronizes /var/lib/zonendpoint/status/signatures.json via SSH
    to %LOCALAPPDATA%\ZonEndpoint\server-status.json every ~15 minutes without
    interrupting the persistent tunnel connection.
    Uses non-interactive SSH flags (-n, BatchMode, ConnectTimeout, ServerAliveInterval)
    and an overall hard execution timeout to prevent process hangs.
    Reads configuration from %LOCALAPPDATA%\ZonEndpoint\tunnel\tunnel-config.json.
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
    $timestamp = [DateTimeOffset]::UtcNow.ToOffset([TimeSpan]::FromHours(7)).ToString("yyyy-MM-dd HH:mm:ss+07:00")
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

$configFile = Join-Path $tunnelDir "tunnel-config.json"
if (-not (Test-Path $configFile)) {
    $msg = "ERROR: Configuration file not found: '$configFile'. Copy 'tunnel-config.example.json' to '$configFile' and set host, user, and keys."
    Write-TunnelLog $msg
    [Console]::Error.WriteLine($msg)
    exit 1
}

try {
    $config = Get-Content -Path $configFile -Raw -Encoding utf8 | ConvertFrom-Json -ErrorAction Stop
} catch {
    $msg = "ERROR: Failed to parse configuration file '$configFile': $($_.Exception.Message)"
    Write-TunnelLog $msg
    [Console]::Error.WriteLine($msg)
    exit 1
}

$missing = @()
if ([string]::IsNullOrWhiteSpace($config.host)) { $missing += "host" }
if ([string]::IsNullOrWhiteSpace($config.user)) { $missing += "user" }
if ([string]::IsNullOrWhiteSpace($config.tunnel_key)) { $missing += "tunnel_key" }
if ([string]::IsNullOrWhiteSpace($config.status_key)) { $missing += "status_key" }
if ($null -eq $config.local_port) { $missing += "local_port" }
if ($null -eq $config.remote_port) { $missing += "remote_port" }

if ($missing.Count -gt 0) {
    $msg = "ERROR: Missing required field(s) in '$configFile': $($missing -join ', ')"
    Write-TunnelLog $msg
    [Console]::Error.WriteLine($msg)
    exit 1
}

$remoteHost = "{0}@{1}" -f $config.user.Trim(), $config.host.Trim()
$tunnelKey = [System.Environment]::ExpandEnvironmentVariables($config.tunnel_key.Trim())
$statusKey = [System.Environment]::ExpandEnvironmentVariables($config.status_key.Trim())
$localPort = [int]$config.local_port
$remotePort = [int]$config.remote_port
$statusSyncIntervalSecs = 900 # 15 minutes
$statusSyncTimeoutSecs = 25   # Hard overall timeout for status ssh call

if (-not (Test-Path $tunnelKey)) {
    $msg = "ERROR: Tunnel private key file not found: '$tunnelKey'"
    Write-TunnelLog $msg
    [Console]::Error.WriteLine($msg)
    exit 1
}

if (-not (Test-Path $statusKey)) {
    $msg = "ERROR: Status private key file not found: '$statusKey'"
    Write-TunnelLog $msg
    [Console]::Error.WriteLine($msg)
    exit 1
}

function Invoke-NonInteractiveSsh {
    param(
        [string]$Key,
        [string]$HostTarget,
        [string]$RemoteCommand = $null,
        [int]$TimeoutSeconds = 25
    )
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "ssh"
    $cmdArg = if ([string]::IsNullOrWhiteSpace($RemoteCommand)) { "" } else { " `"$RemoteCommand`"" }
    $psi.Arguments = "-n -o BatchMode=yes -o ConnectTimeout=15 -o ServerAliveInterval=15 -o ServerAliveCountMax=2 -i `"$Key`" $HostTarget$cmdArg"
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true

    $proc = [System.Diagnostics.Process]::Start($psi)
    $outTask = $proc.StandardOutput.ReadToEndAsync()
    $errTask = $proc.StandardError.ReadToEndAsync()

    if ($proc.WaitForExit($TimeoutSeconds * 1000)) {
        [System.Threading.Tasks.Task]::WaitAll($outTask, $errTask)
        return @{
            ExitCode = $proc.ExitCode
            Output   = $outTask.Result
            Error    = $errTask.Result
            TimedOut = $false
        }
    } else {
        try { $proc.Kill() } catch {}
        return @{
            ExitCode = -1
            Output   = ""
            Error    = "SSH status fetch exceeded hard timeout (${TimeoutSeconds}s)"
            TimedOut = $true
        }
    }
}

function Fetch-ServerStatus {
    param(
        [string]$Key,
        [string]$HostTarget,
        [string]$TargetFile,
        [string]$LogPath,
        [int]$TimeoutSeconds = 25
    )

    $writeLog = {
        param([string]$msg)
        $ts = [DateTimeOffset]::UtcNow.ToOffset([TimeSpan]::FromHours(7)).ToString("yyyy-MM-dd HH:mm:ss+07:00")
        $entry = "[$ts] [StatusSync] $msg"
        try {
            if ($LogPath) {
                Add-Content -Path $LogPath -Value $entry -Encoding utf8
            }
        } catch {}
    }

    try {
        if (-not (Test-Path $Key)) {
            & $writeLog "SSH key file missing, cannot fetch status: $Key"
            return $false
        }

        # Status key has fixed command="cat /var/lib/zonendpoint/status/signatures.json" on server,
        # so no remote command argument is needed.
        $res = Invoke-NonInteractiveSsh -Key $Key -HostTarget $HostTarget -TimeoutSeconds $TimeoutSeconds
        if ($res.TimedOut) {
            & $writeLog "SSH status fetch timed out after ${TimeoutSeconds}s."
            return $false
        }
        if ($res.ExitCode -ne 0 -or [string]::IsNullOrWhiteSpace($res.Output)) {
            & $writeLog ("Failed to retrieve status file from server (exit code {0}): {1}" -f $res.ExitCode, $res.Error)
            return $false
        }

        $rawText = $res.Output.Trim()
        $json = $rawText | ConvertFrom-Json -ErrorAction Stop

        if ($null -eq $json.signatures) {
            & $writeLog "Validation failed: 'signatures' field is missing."
            return $false
        }

        if (-not ($json.signatures -is [int] -or $json.signatures -is [long])) {
            & $writeLog "Validation failed: 'signatures' must be an integer, got $($json.signatures.GetType().FullName)."
            return $false
        }

        $sigLong = [long]$json.signatures
        if ($sigLong -lt 0 -or $sigLong -gt 1000000000) {
            & $writeLog "Validation failed: 'signatures' value $sigLong outside valid range 0..1000000000."
            return $false
        }

        $loadedAtRaw = if ($null -ne $json.loaded_at) { [string]$json.loaded_at } else { [string]$json.loaded_at_utc }
        if ([string]::IsNullOrWhiteSpace($loadedAtRaw)) {
            & $writeLog "Validation failed: 'loaded_at' / 'loaded_at_utc' missing or empty."
            return $false
        }

        $loadedAtTrimmed = $loadedAtRaw.Trim()
        $isoPattern = '^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\+07:00|Z)$'
        if ($loadedAtTrimmed -notmatch $isoPattern) {
            & $writeLog "Validation failed: 'loaded_at' ($loadedAtTrimmed) must match YYYY-MM-DDTHH:MM:SS+07:00 or YYYY-MM-DDTHH:MM:SSZ."
            return $false
        }

        $parsedOffset = [DateTimeOffset]::MinValue
        if (-not [DateTimeOffset]::TryParse($loadedAtTrimmed, [System.Globalization.CultureInfo]::InvariantCulture, [System.Globalization.DateTimeStyles]::None, [ref]$parsedOffset)) {
            & $writeLog "Validation failed: 'loaded_at' ($loadedAtTrimmed) is not a valid date."
            return $false
        }

        $loadedAtNormalized = $parsedOffset.ToOffset([TimeSpan]::FromHours(7)).ToString("yyyy-MM-ddTHH:mm:ss+07:00")
        $fetchedAt = [DateTimeOffset]::UtcNow.ToOffset([TimeSpan]::FromHours(7)).ToString("yyyy-MM-ddTHH:mm:ss+07:00")
        $outPayload = [ordered]@{
            signatures = $sigLong
            loaded_at  = $loadedAtNormalized
            fetched_at = $fetchedAt
            engine     = $json.engine
        }
        $payloadText = $outPayload | ConvertTo-Json -Compress

        $targetDir = Split-Path $TargetFile -Parent
        if (-not (Test-Path $targetDir)) {
            New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
        }

        $tempFile = "$TargetFile.tmp.$PID"
        $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
        [System.IO.File]::WriteAllText($tempFile, $payloadText, $utf8NoBom)
        Move-Item -Path $tempFile -Destination $TargetFile -Force -ErrorAction Stop
        & $writeLog "Successfully updated server status: signatures=$sigLong, loaded_at=$loadedAtNormalized, fetched_at=$fetchedAt"
        return $true
    } catch {
        & $writeLog "Exception during server status sync: $($_.Exception.Message)"
        return $false
    }
}

if ($SyncOnce) {
    Write-TunnelLog "=== ZonEndpoint manual status fetch invoked ==="
    $ok = Fetch-ServerStatus -Key $statusKey -HostTarget $remoteHost -TargetFile $statusFile -LogPath $logFile -TimeoutSeconds $statusSyncTimeoutSecs
    if ($ok) {
        Write-Output "Status fetch succeeded. Target: $statusFile"
        Get-Content $statusFile
    } else {
        Write-Error "Status fetch failed. Check $logFile."
    }
    return
}

Write-TunnelLog "=== ZonEndpoint tunnel supervisor starting (PID: $PID) ==="

# Start background runspace for periodic status sync so ssh -N does not block sync
$runspace = [runspacefactory]::CreateRunspace()
$runspace.Open()
$statusPs = [powershell]::Create()
$statusPs.Runspace = $runspace

# Pass function definitions to runspace to avoid code duplication (S-9)
$helperDefinitions = @"
function Invoke-NonInteractiveSsh {
${function:Invoke-NonInteractiveSsh}
}
function Fetch-ServerStatus {
${function:Fetch-ServerStatus}
}
"@

$syncScript = {
    param($helpers, $keyPath, $remoteHost, $statusFile, $logFile, $intervalSecs, $timeoutSecs)

    . ([scriptblock]::Create($helpers))

    # Initial sync on launch
    [void](Fetch-ServerStatus -Key $keyPath -HostTarget $remoteHost -TargetFile $statusFile -LogPath $logFile -TimeoutSeconds $timeoutSecs)

    while ($true) {
        Start-Sleep -Seconds $intervalSecs
        [void](Fetch-ServerStatus -Key $keyPath -HostTarget $remoteHost -TargetFile $statusFile -LogPath $logFile -TimeoutSeconds $timeoutSecs)
    }
}

[void]$statusPs.AddScript($syncScript).AddArgument($helperDefinitions).AddArgument($statusKey).AddArgument($remoteHost).AddArgument($statusFile).AddArgument($logFile).AddArgument($statusSyncIntervalSecs).AddArgument($statusSyncTimeoutSecs)
$statusAsyncHandle = $statusPs.BeginInvoke()
Write-TunnelLog "Started background server-status sync runspace (interval: ${statusSyncIntervalSecs}s, timeout: ${statusSyncTimeoutSecs}s)."

$backoff = 5
$maxBackoff = 60

try {
    while ($true) {
        Write-TunnelLog "Connecting SSH tunnel (127.0.0.1:$localPort -> 127.0.0.1:$remotePort via $remoteHost)..."
        $start = Get-Date

        try {
            & ssh -i $tunnelKey `
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
