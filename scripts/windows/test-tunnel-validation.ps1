<#
.SYNOPSIS
    Self-testing unit suite for ZonEndpoint tunnel configuration validation (Task V-64 / Q-43).
.DESCRIPTION
    Runs offline (no network required) tests against zon-tunnel.ps1 to verify that:
    - Host containing whitespace or "-oProxyCommand=..." is rejected
    - Port 0, 70000, "22 -R", string ports, or decimal ports are rejected
    - Invalid/weird usernames (capital letters, symbols, injections) are rejected
    - Missing keys or key paths containing quotes are rejected
    - Missing required fields are rejected
    - Valid configurations (IPv4, IPv6, hostname) are accepted
    - Invalid config causes immediate exit 1 with clear error in log and console without fallback
#>

param(
    [switch]$VerboseOutput
)

$ErrorActionPreference = 'Continue'
$scriptRoot = $PSScriptRoot
$zonTunnelScript = Join-Path $scriptRoot "zon-tunnel.ps1"

if (-not (Test-Path $zonTunnelScript)) {
    Write-Error "zon-tunnel.ps1 not found at: $zonTunnelScript"
    exit 1
}

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "zon-validation-test-$PID"
if (Test-Path $tempRoot) {
    Remove-Item -Path $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null

$dummyTunnelKey = Join-Path $tempRoot "dummy_tunnel_key"
$dummyStatusKey = Join-Path $tempRoot "dummy_status_key"
Set-Content -Path $dummyTunnelKey -Value "dummy-private-key-data" -Encoding utf8
Set-Content -Path $dummyStatusKey -Value "dummy-status-key-data" -Encoding utf8

$testCases = @(
    # --- Host tests ---
    @{
        Name = "Host with trailing whitespace"
        Config = @{ host = "100.96.250.245 "; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'host'"
    },
    @{
        Name = "Host with leading whitespace"
        Config = @{ host = " 100.96.250.245"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'host'"
    },
    @{
        Name = "Host with whitespace in middle"
        Config = @{ host = "100.96. 250.245"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'host'"
    },
    @{
        Name = "Host with spaces (host name)"
        Config = @{ host = "host name"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'host'"
    },
    @{
        Name = "Host with -oProxyCommand injection"
        Config = @{ host = "-oProxyCommand=calc.exe"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'host'"
    },
    @{
        Name = "Host starting with hyphen (-h)"
        Config = @{ host = "-h"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'host'"
    },
    @{
        Name = "Host with invalid numeric IPv4 (999.999.999.999)"
        Config = @{ host = "999.999.999.999"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'host'"
    },
    @{
        Name = "Host empty string"
        Config = @{ host = ""; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Missing required field"
    },

    # --- Port tests ---
    @{
        Name = "local_port = 0"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = 0; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'local_port'"
    },
    @{
        Name = "remote_port = 0"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = 13314; remote_port = 0; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'remote_port'"
    },
    @{
        Name = "local_port = 70000"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = 70000; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'local_port'"
    },
    @{
        Name = "remote_port = 70000"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = 13314; remote_port = 70000; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'remote_port'"
    },
    @{
        Name = "local_port string injection ('22 -R')"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = "22 -R"; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'local_port'"
    },
    @{
        Name = "local_port string ('13314')"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = "13314"; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'local_port'"
    },
    @{
        Name = "remote_port string ('3310')"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = 13314; remote_port = "3310"; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'remote_port'"
    },
    @{
        Name = "local_port decimal (13314.5)"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = 13314.5; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'local_port'"
    },
    @{
        Name = "remote_port decimal (3310.5)"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = 13314; remote_port = 3310.5; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'remote_port'"
    },

    # --- User tests ---
    @{
        Name = "User uppercase ('User')"
        Config = @{ host = "100.96.250.245"; user = "User"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'user'"
    },
    @{
        Name = "User command injection ('root; rm -rf /')"
        Config = @{ host = "100.96.250.245"; user = "root; rm -rf /"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'user'"
    },
    @{
        Name = "User starting with hyphen ('-user')"
        Config = @{ host = "100.96.250.245"; user = "-user"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'user'"
    },
    @{
        Name = "User starting with digit ('1user')"
        Config = @{ host = "100.96.250.245"; user = "1user"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'user'"
    },
    @{
        Name = "User with spaces ('user name')"
        Config = @{ host = "100.96.250.245"; user = "user name"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'user'"
    },
    @{
        Name = "User empty string"
        Config = @{ host = "100.96.250.245"; user = ""; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Missing required field"
    },

    # --- Key tests ---
    @{
        Name = "tunnel_key nonexistent file"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = "C:\nonexistent\tunnel_key"; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'tunnel_key'"
    },
    @{
        Name = "status_key nonexistent file"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = "C:\nonexistent\status_key" }
        ExpectValid = $false
        ErrorMatch = "Invalid 'status_key'"
    },
    @{
        Name = "tunnel_key path containing quotes"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = "$dummyTunnelKey`""; status_key = $dummyStatusKey }
        ExpectValid = $false
        ErrorMatch = "Invalid 'tunnel_key'"
    },
    @{
        Name = "status_key path containing quotes"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = "$dummyStatusKey`"" }
        ExpectValid = $false
        ErrorMatch = "Invalid 'status_key'"
    },

    # --- Valid configuration tests ---
    @{
        Name = "Valid config with IPv4"
        Config = @{ host = "100.96.250.245"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $true
        ErrorMatch = $null
    },
    @{
        Name = "Valid config with loopback IPv4"
        Config = @{ host = "127.0.0.1"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $true
        ErrorMatch = $null
    },
    @{
        Name = "Valid config with IPv6 (::1)"
        Config = @{ host = "::1"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $true
        ErrorMatch = $null
    },
    @{
        Name = "Valid config with hostname (clamav.lab)"
        Config = @{ host = "clamav.lab"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
        ExpectValid = $true
        ErrorMatch = $null
    }
)

$passCount = 0
$failCount = 0

Write-Host "============================================================"
Write-Host " Running ZonEndpoint Tunnel Config Validation Test Suite"
Write-Host " Total test cases: $($testCases.Count)"
Write-Host "============================================================"

for ($i = 0; $i -lt $testCases.Count; $i++) {
    $tc = $testCases[$i]
    $testDir = Join-Path $tempRoot "tc-$i"
    $tunnelSubdir = Join-Path (Join-Path $testDir "ZonEndpoint") "tunnel"
    New-Item -ItemType Directory -Path $tunnelSubdir -Force | Out-Null

    $configFile = Join-Path $tunnelSubdir "tunnel-config.json"
    $tc.Config | ConvertTo-Json | Set-Content -Path $configFile -Encoding utf8

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "powershell.exe"
    $psi.Arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$zonTunnelScript`" -ValidateOnly -AppDataRoot `"$testDir`""
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true

    $proc = [System.Diagnostics.Process]::Start($psi)
    $stdout = $proc.StandardOutput.ReadToEnd()
    $stderr = $proc.StandardError.ReadToEnd()
    $proc.WaitForExit()
    $code = $proc.ExitCode

    $logFile = Join-Path $tunnelSubdir "tunnel.log"
    $logContent = if (Test-Path $logFile) { Get-Content -Path $logFile -Raw -Encoding utf8 } else { "" }

    $passed = $false
    if ($tc.ExpectValid) {
        if ($code -eq 0 -and ($stdout -like "*Configuration*is valid*")) {
            $passed = $true
        } else {
            $failReason = "Expected exit code 0 and valid message, got code=$code. StdErr: $stderr"
        }
    } else {
        if ($code -eq 1) {
            if ($null -ne $tc.ErrorMatch) {
                if ($stderr -like "*$($tc.ErrorMatch)*" -or $logContent -like "*$($tc.ErrorMatch)*") {
                    $passed = $true
                } else {
                    $failReason = "Exit code 1 OK, but expected error message containing '$($tc.ErrorMatch)'. Got stderr: '$stderr', log: '$logContent'"
                }
            } else {
                $passed = $true
            }
        } else {
            $failReason = "Expected exit code 1 for invalid config, got code=$code. StdOut: $stdout"
        }
    }

    if ($passed) {
        $passCount++
        Write-Host (" [PASS] #{0:D2}: {1}" -f ($i + 1), $tc.Name) -ForegroundColor Green
    } else {
        $failCount++
        Write-Host (" [FAIL] #{0:D2}: {1} -> {2}" -f ($i + 1), $tc.Name, $failReason) -ForegroundColor Red
    }
}

# Also test supervisor mode exit without -ValidateOnly on invalid config
Write-Host "`nTesting supervisor mode rejection (no -ValidateOnly)..."
$testDirSuper = Join-Path $tempRoot "tc-super-invalid"
$tunnelSubdirSuper = Join-Path (Join-Path $testDirSuper "ZonEndpoint") "tunnel"
New-Item -ItemType Directory -Path $tunnelSubdirSuper -Force | Out-Null
$badConfig = @{ host = "-oProxyCommand=calc.exe"; user = "zontunnel"; local_port = 13314; remote_port = 3310; tunnel_key = $dummyTunnelKey; status_key = $dummyStatusKey }
$badConfig | ConvertTo-Json | Set-Content -Path (Join-Path $tunnelSubdirSuper "tunnel-config.json") -Encoding utf8

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = "powershell.exe"
$psi.Arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$zonTunnelScript`" -AppDataRoot `"$testDirSuper`""
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true

$proc = [System.Diagnostics.Process]::Start($psi)
$stdout = $proc.StandardOutput.ReadToEnd()
$stderr = $proc.StandardError.ReadToEnd()
$proc.WaitForExit()
$code = $proc.ExitCode

if ($code -eq 1 -and ($stderr -like "*Invalid 'host'*")) {
    $passCount++
    Write-Host " [PASS] Supervisor mode exited immediately with code 1 on invalid config." -ForegroundColor Green
} else {
    $failCount++
    Write-Host " [FAIL] Supervisor mode did not exit 1 as expected. Code=$code, stderr=$stderr" -ForegroundColor Red
}

# Clean up temporary test files
Remove-Item -Path $tempRoot -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "============================================================"
Write-Host " Test Summary: $passCount passed, $failCount failed."
Write-Host "============================================================"

if ($failCount -gt 0) {
    exit 1
} else {
    exit 0
}
