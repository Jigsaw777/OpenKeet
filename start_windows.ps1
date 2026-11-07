$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Set-Location -LiteralPath $PSScriptRoot

$mutex = New-Object Threading.Mutex($false, 'PersonalInterviewAssistantSetup')
try {
    $ownsSetup = $mutex.WaitOne(0)
} catch [Threading.AbandonedMutexException] {
    $ownsSetup = $true
}
if (-not $ownsSetup) {
    $mutex.Dispose()
    return
}

function Add-DesktopShortcut {
    $desktop = [Environment]::GetFolderPath('DesktopDirectory')
    if (-not $desktop) { return }
    $shortcutPath = Join-Path $desktop 'OpenKeet (Legacy).lnk'
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($shortcutPath)
    $description = 'Launch the legacy OpenKeet prototype'
    if ((Test-Path -LiteralPath $shortcutPath) -and $shortcut.Description -ne $description) { return }
    $launcher = Join-Path $PSScriptRoot 'start_windows.vbs'
    if ($shortcut.Arguments -eq ('"' + $launcher + '"')) { return }
    $shortcut.TargetPath = Join-Path $env:SystemRoot 'System32\wscript.exe'
    $shortcut.Arguments = '"' + $launcher + '"'
    $shortcut.WorkingDirectory = $PSScriptRoot
    $shortcut.Description = $description
    $shortcut.IconLocation = (Join-Path $env:SystemRoot 'System32\mmres.dll') + ',0'
    $shortcut.Save()
}

function Invoke-SetupProcess($executable, [string[]]$commandArgs) {
    $startInfo = New-Object Diagnostics.ProcessStartInfo
    $startInfo.FileName = $executable
    $startInfo.Arguments = $commandArgs -join ' '
    $startInfo.WorkingDirectory = $PSScriptRoot
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($startInfo)
    try {
        $outputTask = $process.StandardOutput.ReadToEndAsync()
        $errorTask = $process.StandardError.ReadToEndAsync()
        while (-not $process.WaitForExit(100)) {
            [Windows.Forms.Application]::DoEvents()
        }
        [IO.File]::WriteAllText($setupOutput, $outputTask.GetAwaiter().GetResult())
        [IO.File]::WriteAllText($setupError, $errorTask.GetAwaiter().GetResult())
        return $process.ExitCode
    } finally {
        $process.Dispose()
    }
}

function Install-WithWinGet($package) {
    $winget = Get-Command winget.exe -ErrorAction SilentlyContinue
    if (-not $winget) { throw 'WinGet is unavailable. Install Windows App Installer, then open the shortcut again.' }
    $installArgs = @('install', '--id', $package, '--exact', '--source', 'winget', '--scope', 'user',
        '--silent', '--disable-interactivity', '--accept-source-agreements', '--accept-package-agreements')
    if ((Invoke-SetupProcess $winget.Source $installArgs) -ne 0) {
        throw "WinGet could not install $package."
    }
}

$progress = $null
$logFolder = Join-Path $env:LOCALAPPDATA 'Personal Interview Assistant'
$setupOutput = Join-Path $logFolder 'setup-output.log'
$setupError = Join-Path $logFolder 'setup-error.log'
try {
    try { Add-DesktopShortcut } catch { Write-Warning $_.Exception.Message }

    $venvPython = Join-Path $PSScriptRoot '.venv\Scripts\python.exe'
    $venvGui = Join-Path $PSScriptRoot '.venv\Scripts\pythonw.exe'
    $stamp = Join-Path $PSScriptRoot '.venv\assistant-requirements.sha256'
    $hasher = [Security.Cryptography.SHA256]::Create()
    try {
        $requirements = [IO.File]::ReadAllBytes((Join-Path $PSScriptRoot 'requirements.txt'))
        $requirementsHash = [Convert]::ToBase64String($hasher.ComputeHash($requirements))
    } finally {
        $hasher.Dispose()
    }
    $setupNeeded = -not ((Test-Path -LiteralPath $venvGui) -and (Test-Path -LiteralPath $venvPython) -and
        (Test-Path -LiteralPath $stamp) -and (Get-Content -LiteralPath $stamp -Raw).Trim() -eq $requirementsHash)
    $wingetLinks = Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Links'
    $nativeClaude = Join-Path $env:USERPROFILE '.local\bin'
    $env:Path += ";$wingetLinks;$nativeClaude"
    $claudeMissing = -not (Get-Command claude -CommandType Application -ErrorAction SilentlyContinue)
    if ($setupNeeded -or $claudeMissing) {
        New-Item -ItemType Directory -Path $logFolder -Force | Out-Null
        $progress = New-Object Windows.Forms.Form
        $progress.Text = 'Application setup'
        $progress.StartPosition = 'CenterScreen'
        $progress.ClientSize = New-Object Drawing.Size(400, 80)
        $progress.FormBorderStyle = 'FixedDialog'
        $progress.ControlBox = $false
        $label = New-Object Windows.Forms.Label
        $label.Dock = 'Fill'
        $label.TextAlign = 'MiddleCenter'
        $label.Text = 'Preparing Python...'
        $progress.Controls.Add($label)
        $progress.Show()
        $progress.Refresh()
    }

    if ($setupNeeded) {
        $probe = '"import sys, tkinter; assert sys.version_info[:2] in [(3,11),(3,12)] and sys.maxsize > 2**32"'
        if (-not (Test-Path -LiteralPath $venvPython)) {
            $launcher = Get-Command py.exe -ErrorAction SilentlyContinue
            $created = $false
            if ($launcher) {
                foreach ($version in @('-3.12', '-3.11')) {
                    if ((Invoke-SetupProcess $launcher.Source @($version, '-c', $probe)) -eq 0) {
                        $created = (Invoke-SetupProcess $launcher.Source @($version, '-m', 'venv', '.venv')) -eq 0
                        break
                    }
                }
            }
            if (-not $created) {
                $label.Text = 'Installing Python 3.12...'
                $progress.Refresh()
                $pythonKey = 'HKCU:\Software\Python\PythonCore\3.12\InstallPath'
                if (-not (Test-Path -LiteralPath $pythonKey)) { Install-WithWinGet 'Python.Python.3.12' }
                $installedPython = (Get-Item -LiteralPath $pythonKey).GetValue('ExecutablePath')
                if (-not $installedPython) {
                    $installedPython = Join-Path (Get-Item -LiteralPath $pythonKey).GetValue('') 'python.exe'
                }
                if ((Invoke-SetupProcess $installedPython @('-m', 'venv', '.venv')) -ne 0) {
                    throw 'The local Python environment could not be created.'
                }
            }
        }
        if ((Invoke-SetupProcess $venvPython @('-c', $probe)) -ne 0) {
            throw 'The existing .venv needs 64-bit Python 3.11 or 3.12 with Tk. Rename .venv and open the shortcut again.'
        }
        $label.Text = 'Installing app dependencies. This may take a few minutes...'
        $progress.Refresh()
        if ((Invoke-SetupProcess $venvPython @('-m', 'pip', 'install', '--disable-pip-version-check', '-r', 'requirements.txt')) -ne 0) {
            throw 'Python dependencies could not be installed. Check your internet connection and try again.'
        }
        Set-Content -LiteralPath $stamp -Value $requirementsHash -Encoding ASCII
    }

    if ($claudeMissing) {
        $label.Text = 'Installing Claude Code...'
        $progress.Refresh()
        try { Install-WithWinGet 'Anthropic.ClaudeCode' } catch {
            [Windows.Forms.MessageBox]::Show($_.Exception.Message + "`nYou can still open the app and choose another provider.",
                'OpenKeet (Legacy)') | Out-Null
        }
    }
    Start-Process -FilePath $venvGui -ArgumentList @('app.py', '--prepare') -WorkingDirectory $PSScriptRoot
} catch {
    [Windows.Forms.MessageBox]::Show($_.Exception.Message + "`nSetup logs are in $logFolder.",
        'OpenKeet (Legacy)') | Out-Null
    exit 1
} finally {
    if ($progress) { $progress.Dispose() }
    $mutex.ReleaseMutex()
    $mutex.Dispose()
}
