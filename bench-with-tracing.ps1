#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Runs tracelogging vs WPP Criterion benchmarks with active ETW listeners
    and prints a summary comparison table.

.DESCRIPTION
    Starts ETW trace sessions for both providers, runs cargo bench, parses
    Criterion output, then prints a side-by-side table. Requires administrator
    privileges for ETW session management.

.EXAMPLE
    .\bench-with-tracing.ps1
    .\bench-with-tracing.ps1 -Filter "multi_field"
#>

param(
    [string]$Filter  # Optional Criterion filter (e.g. "multi_field", "str_field")
)

$ErrorActionPreference = "Stop"

# Ensure MSVC build tools are on PATH. If the MSVC link.exe isn't found,
# locate vcvars64.bat and import the developer environment.
$msvcLink = Get-Command "link.exe" -ErrorAction SilentlyContinue |
    Where-Object { $_.Source -match "MSVC" }
if (-not $msvcLink) {
    # Search common VS install locations for vcvars64.bat.
    $vcvars = Get-ChildItem "C:\Program Files*\Microsoft Visual Studio" -Recurse `
        -Filter "vcvars64.bat" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1 -ExpandProperty FullName

    if ($vcvars) {
        Write-Host "Loading MSVC environment from $vcvars ..." -ForegroundColor Yellow
        cmd /c "`"$vcvars`" >nul 2>&1 && set" | ForEach-Object {
            if ($_ -match '^([^=]+)=(.*)$') {
                [System.Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], "Process")
            }
        }
    } else {
        Write-Error "Could not find vcvars64.bat. Install Visual Studio with the 'Desktop development with C++' workload."
    }
}

$TlgGuid  = "{bf23fbc9-fdb4-5ff0-224a-d48fce07abc4}"  # BenchProvider.TraceLogging
$WppGuid  = "{84bdb2e9-829e-41b3-b891-02f454bc2bd7}"  # WppBench
$TlgSession = "BenchTlg"
$WppSession = "BenchWpp"

function Stop-Sessions {
    logman stop $TlgSession 2>$null | Out-Null
    logman delete $TlgSession 2>$null | Out-Null
    logman stop $WppSession 2>$null | Out-Null
    logman delete $WppSession 2>$null | Out-Null
}

# Clean up any leftover sessions from a previous run.
Stop-Sessions

try {
    Write-Host "Starting ETW trace sessions..." -ForegroundColor Cyan
    logman create trace $TlgSession -o tlg.etl -p $TlgGuid 0xFF 5 -ow -f bincirc -max 256 | Out-Null
    logman start $TlgSession | Out-Null
    Write-Host "  [+] $TlgSession listening on $TlgGuid"

    logman create trace $WppSession -o wpp.etl -p $WppGuid 0xFF 5 -ow -f bincirc -max 256 | Out-Null
    logman start $WppSession | Out-Null
    Write-Host "  [+] $WppSession listening on $WppGuid"

    Write-Host ""
    Write-Host "Running benchmarks..." -ForegroundColor Cyan
    Write-Host ""

    $benchArgs = @()
    if ($Filter) {
        $benchArgs = @("--", $Filter)
    }

    $output = rustup run stage1 cargo bench @benchArgs 2>&1 | Out-String
    Write-Host $output

    if ($LASTEXITCODE -ne 0) {
        Write-Host "Benchmark run failed (exit code $LASTEXITCODE)" -ForegroundColor Red
        return
    }

    # Parse Criterion output. Criterion may print the name and time on the
    # same line or split across two lines when the name is long:
    #   u32_field/wpp           time:   [...]        (single line)
    #   enabled_check/tracelogging                   (name only)
    #                           time:   [...]        (time on next line)
    $results = @{}
    $timePattern = 'time:\s+\[[\d.]+ \w+ (?<estimate>[\d.]+) (?<unit>\w+) [\d.]+ \w+\]'
    $namePattern = '(?<group>\S+)/(?<impl>tracelogging|wpp)'
    $pendingName = $null

    foreach ($line in $output -split "`n") {
        if ($line -match "$namePattern\s+$timePattern") {
            $group = $Matches['group']
            $impl  = $Matches['impl']
            $value = "$($Matches['estimate']) $($Matches['unit'])"
            if (-not $results.ContainsKey($group)) { $results[$group] = @{} }
            $results[$group][$impl] = $value
            $pendingName = $null
        }
        elseif ($line -match "^\s*$namePattern\s*$") {
            $pendingName = @{ group = $Matches['group']; impl = $Matches['impl'] }
        }
        elseif ($pendingName -and $line -match $timePattern) {
            $group = $pendingName.group
            $impl  = $pendingName.impl
            $value = "$($Matches['estimate']) $($Matches['unit'])"
            if (-not $results.ContainsKey($group)) { $results[$group] = @{} }
            $results[$group][$impl] = $value
            $pendingName = $null
        }
        else {
            $pendingName = $null
        }
    }

    if ($results.Count -eq 0) {
        Write-Host "No benchmark results parsed." -ForegroundColor Yellow
        return
    }

    # Print comparison table.
    Write-Host ""
    Write-Host "=== Results ===" -ForegroundColor Green
    Write-Host ""

    $nameWidth = ($results.Keys | ForEach-Object { $_.Length } | Measure-Object -Maximum).Maximum
    $nameWidth = [Math]::Max($nameWidth, 10)
    $colWidth  = 18

    $header = "{0,-$nameWidth}  {1,-$colWidth}  {2,-$colWidth}" -f "Benchmark", "tracelogging", "wpp"
    $separator = "-" * $header.Length
    Write-Host $header -ForegroundColor White
    Write-Host $separator

    # Collect group order from the parsed results (matches the order they appeared).
    $orderedGroups = [System.Collections.Generic.List[string]]::new()
    $namePattern2 = '(?<group>\S+)/(?<impl>tracelogging|wpp)'
    foreach ($line in $output -split "`n") {
        if ($line -match $namePattern2) {
            $g = $Matches['group']
            if (-not $orderedGroups.Contains($g)) {
                $orderedGroups.Add($g)
            }
        }
    }

    foreach ($group in $orderedGroups) {
        $tlg = if ($results[$group].ContainsKey('tracelogging')) { $results[$group]['tracelogging'] } else { "-" }
        $wpp = if ($results[$group].ContainsKey('wpp'))          { $results[$group]['wpp'] }          else { "-" }
        $row = "{0,-$nameWidth}  {1,-$colWidth}  {2,-$colWidth}" -f $group, $tlg, $wpp
        Write-Host $row
    }
    Write-Host ""
}
finally {
    Write-Host "Stopping ETW trace sessions..." -ForegroundColor Cyan
    Stop-Sessions
    Write-Host "  [+] Sessions stopped and cleaned up."

    # Clean up .etl files
    Remove-Item -Path "*.etl" -ErrorAction SilentlyContinue
    Write-Host "  [+] Removed .etl files."
}
