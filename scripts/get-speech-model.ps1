param(
    [ValidateSet('small', 'turbo')][string]$Model = 'small',
    [string]$Directory = (Join-Path $PSScriptRoot '../models')
)

$ErrorActionPreference = 'Stop'
$revision = '5359861c739e955e79d9a303bcbc70fb988958b1'
$filename, $expected = switch ($Model) {
    'small' { 'ggml-small.en.bin', 'c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d' }
    'turbo' { 'ggml-large-v3-turbo-q5_0.bin', '394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2' }
}

function Get-ModelHash([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    $hash = [Security.Cryptography.SHA256]::Create()
    try { [BitConverter]::ToString($hash.ComputeHash($stream)).Replace('-', '') }
    finally { $stream.Dispose(); $hash.Dispose() }
}

$folder = New-Item -ItemType Directory -Path $Directory -Force
$path = Join-Path $folder.FullName $filename
if (Test-Path -LiteralPath $path) {
    if ((Get-ModelHash $path) -ne $expected) { throw "Existing model failed SHA-256 verification: $path" }
} else {
    $partial = "$path.part"
    curl.exe --fail --location --progress-bar --retry 2 "https://huggingface.co/ggerganov/whisper.cpp/resolve/$revision/$filename" --output $partial
    if ($LASTEXITCODE -ne 0) { throw 'Model download failed. Run the command again to retry.' }
    if ((Get-ModelHash $partial) -ne $expected) { throw 'Model checksum did not match. Run the command again to retry.' }
    Move-Item -LiteralPath $partial -Destination $path
}
Write-Output "Verified $path"
