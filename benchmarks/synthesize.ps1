param(
    [Parameter(Mandatory)][string]$Jobs,
    [Parameter(Mandatory)][string]$Output
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$synth = [System.Speech.Synthesis.SpeechSynthesizer]::new()
$format = [System.Speech.AudioFormat.SpeechAudioFormatInfo]::new(
    16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen,
    [System.Speech.AudioFormat.AudioChannel]::Mono
)
try {
    foreach ($job in (Get-Content -Raw -Encoding UTF8 -LiteralPath $Jobs | ConvertFrom-Json)) {
        $synth.SelectVoice($job.voice)
        $synth.Rate = $job.rate
        $synth.SetOutputToWaveFile((Join-Path $Output $job.file), $format)
        $synth.Speak($job.text)
        $synth.SetOutputToNull()
    }
} finally {
    $synth.Dispose()
}
