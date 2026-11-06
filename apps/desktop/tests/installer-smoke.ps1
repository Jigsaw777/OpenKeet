param(
    [string]$Installer = (Join-Path $PSScriptRoot '../../../target/release/bundle/nsis/Personal Copilot_0.1.0_x64-setup.exe')
)

$ErrorActionPreference = 'Stop'
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall'
$existing = Get-ChildItem $uninstallKey -ErrorAction SilentlyContinue |
    ForEach-Object { Get-ItemProperty -LiteralPath $_.PSPath -ErrorAction SilentlyContinue } |
    Where-Object { $_.DisplayName -eq 'Personal Copilot' }
if ($existing) { throw 'Personal Copilot is already installed. Run the portable native smoke test instead.' }

$installerPath = (Resolve-Path -LiteralPath $Installer).Path
$target = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../target')).Path
$installDir = Join-Path $target ('installed-smoke-' + [Guid]::NewGuid().ToString('N'))
if (![IO.Path]::GetFullPath($installDir).StartsWith($target + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Unexpected temporary install path.'
}
$installed = Start-Process -FilePath $installerPath -ArgumentList @('/S', '/NS', "/D=$installDir") -WindowStyle Hidden -Wait -PassThru
if ($installed.ExitCode -ne 0) { throw "Installer exited with $($installed.ExitCode)." }

$application = Join-Path $installDir 'personal-copilot-desktop.exe'
$uninstaller = (Resolve-Path -LiteralPath (Join-Path $installDir 'uninstall.exe')).Path
if (![IO.Path]::GetDirectoryName($uninstaller).Equals($installDir, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Unexpected uninstaller path.'
}
$previousExecutable = $env:COPILOT_DESKTOP_EXE
try {
    $env:COPILOT_DESKTOP_EXE = (Resolve-Path -LiteralPath $application).Path
    npm.cmd --prefix (Join-Path $PSScriptRoot '..') run test:native
    $smoke = $LASTEXITCODE
} finally {
    $env:COPILOT_DESKTOP_EXE = $previousExecutable
    $uninstalled = Start-Process -FilePath $uninstaller -ArgumentList '/S' -WindowStyle Hidden -Wait -PassThru
    if ($uninstalled.ExitCode -ne 0 -or (Test-Path -LiteralPath $application)) {
        throw 'Temporary app uninstall did not complete.'
    }
    Write-Output 'Temporary Windows install and uninstall completed.'
}
exit $smoke
