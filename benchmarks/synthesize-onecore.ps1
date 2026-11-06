param(
    [Parameter(Mandatory)][string]$Jobs,
    [Parameter(Mandatory)][string]$Output
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$synthType = [Windows.Media.SpeechSynthesis.SpeechSynthesizer, Windows.Media.SpeechSynthesis, ContentType=WindowsRuntime]
$streamType = [Windows.Media.SpeechSynthesis.SpeechSynthesisStream, Windows.Media.SpeechSynthesis, ContentType=WindowsRuntime]
$asTask = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
        $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
} | Select-Object -First 1).MakeGenericMethod($streamType)
$voices = @{}
foreach ($voice in $synthType::AllVoices) { $voices[$voice.DisplayName] = $voice }
$synth = [Activator]::CreateInstance($synthType)
try {
    foreach ($job in (Get-Content -Raw -Encoding UTF8 -LiteralPath $Jobs | ConvertFrom-Json)) {
        if (!$voices.ContainsKey($job.voice)) { throw "Install the Windows voice $($job.voice) to reproduce this pack." }
        $synth.Voice = $voices[$job.voice]
        $synth.Options.SpeakingRate = [double]$job.rate
        $result = $asTask.Invoke($null, @($synth.SynthesizeTextToStreamAsync($job.text))).GetAwaiter().GetResult()
        try {
            $inputStream = [System.IO.WindowsRuntimeStreamExtensions]::AsStreamForRead($result)
            try {
                $file = [IO.File]::Create((Join-Path $Output $job.file))
                try { $inputStream.CopyTo($file) }
                finally { $file.Dispose() }
            } finally { $inputStream.Dispose() }
        } finally { $result.Dispose() }
    }
} finally { $synth.Dispose() }
