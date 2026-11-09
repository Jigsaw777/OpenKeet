param(
    [Parameter(Mandatory = $true)][string]$Before,
    [Parameter(Mandatory = $true)][string]$After,
    [ValidateRange(1, 10)][int]$Repetitions = 3,
    [switch]$StartupOnly,
    [string]$Output = 'benchmarks/output/desktop-startup/results.json'
)

$ErrorActionPreference = 'Stop'
$paths = [ordered]@{
    B = (Resolve-Path -LiteralPath $Before).ProviderPath
    C = (Resolve-Path -LiteralPath $After).ProviderPath
}
$profiles = Join-Path (Split-Path -Parent $PSCommandPath) ('output/desktop-startup/run-' + [Guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $profiles

function Measure-Desktop([string]$Executable, [string]$Profile) {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $Executable
    $info.WorkingDirectory = Split-Path -Parent $Executable
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.EnvironmentVariables['WEBVIEW2_USER_DATA_FOLDER'] = $Profile
    $info.EnvironmentVariables['COPILOT_DATA_DIR'] = Join-Path $Profile 'application'
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $app = [Diagnostics.Process]::Start($info)
    try {
        do {
            if ($app.HasExited) { throw "Application exited during startup ($($app.ExitCode))." }
            if ($clock.ElapsedMilliseconds -gt 15000) { throw 'Timed out waiting for the native window.' }
            $app.Refresh()
            if ($app.MainWindowHandle -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 20 }
        } until ($app.MainWindowHandle -ne [IntPtr]::Zero)
        $windowMs = $clock.ElapsedMilliseconds

        Start-Sleep -Milliseconds 750
        $rows = @(Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, CreationDate |
            Where-Object { $_.CreationDate -ge $app.StartTime })
        $ids = [Collections.Generic.HashSet[int]]::new()
        [void]$ids.Add($app.Id)
        do {
            $count = $ids.Count
            foreach ($row in $rows) {
                if ($ids.Contains([int]$row.ParentProcessId)) { [void]$ids.Add([int]$row.ProcessId) }
            }
        } while ($ids.Count -ne $count)
        $members = @(Get-Process -Id ([int[]]$ids) -ErrorAction SilentlyContinue)
        $resident = ($members | Measure-Object -Property WorkingSet64 -Sum).Sum
        $private = ($members | Measure-Object -Property PrivateMemorySize64 -Sum).Sum

        $shutdownMs = $null
        if (!$StartupOnly) {
            $clock.Restart()
            do {
                if ($app.HasExited) { throw 'The process exited before the shutdown probe.' }
                $app.Refresh()
                $closeSent = $app.CloseMainWindow()
                if (!$closeSent) { Start-Sleep -Milliseconds 50 }
            } until ($closeSent -or $clock.ElapsedMilliseconds -ge 2000)
            if (!$closeSent) { throw 'Could not send close to a native main window.' }
            if (!$app.WaitForExit(5000)) { throw 'The process stayed open after the close request.' }
            $shutdownMs = $clock.ElapsedMilliseconds
        }
        [pscustomobject]@{
            window_ms = $windowMs
            shutdown_ms = $shutdownMs
            processes = $members.Count
            summed_working_set_mib = [Math]::Round($resident / 1MB, 1)
            summed_private_mib = [Math]::Round($private / 1MB, 1)
        }
    } finally {
        if (!$app.HasExited) { $app.Kill(); $null = $app.WaitForExit(5000) }
        $app.Dispose()
    }
}

$results = foreach ($condition in @('fresh-webview-profile', 'reused-webview-profile')) {
    for ($run = 1; $run -le $Repetitions; $run++) {
        $order = if ($run % 2 -eq 0) { @('C', 'B') } else { @('B', 'C') }
        foreach ($label in $order) {
            $profileRun = if ($condition -eq 'fresh-webview-profile') { $run } else { 1 }
            $profile = Join-Path $profiles "$label-$profileRun"
            try {
                $sample = Measure-Desktop $paths[$label] $profile
                [pscustomobject]@{ build = $label; condition = $condition; run = $run; sample = $sample; error = $null }
            } catch {
                [pscustomobject]@{ build = $label; condition = $condition; run = $run; sample = $null; error = $_.Exception.Message }
            }
        }
    }
}
$results | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $Output -Encoding UTF8
$results | ForEach-Object {
    '{0} {1} #{2}: {3}' -f $_.build, $_.condition, $_.run, $(if ($_.sample) { $_.sample | ConvertTo-Json -Compress } else { $_.error })
}
if ($results | Where-Object { $_.error }) { exit 1 }
