# Interview speech stress test

## Corpus and scoring

Run on Windows 11 x64 with an Intel i5-1335U and 16 GiB RAM. The pack contains 112 spoken clips and four non-speech controls, with 537.23 seconds of audio and 960 reference words. Eight offline Windows voices cover US, UK, and Indian English locales. This measures synthetic speech. Human speakers, conferencing applications, and mixed-room recordings need separate tests. [Windows speech synthesis](https://learn.microsoft.com/en-us/uwp/api/windows.media.speechsynthesis.speechsynthesizer)

The recipe covers tongue twisters at normal and 1.65× synthesis speed, homophones, a quiet peak of 0.008, 8 dB added noise, a simulated 8 kHz telephone path, an 80 ms clipped onset, short “Why?” prompts, and corrections across a 1.6-second pause. Controls include silence, a tone, synthetic keyboard clicks, and synthesized chords. All waveforms stay in the ignored output folder. The run used manifest SHA-256 `b79cb51873988fd561a441ed6b00cc4a0dd707c0e4c5125f7f888dee14fc3893`.

WER uses the same case-folded, punctuation-insensitive scorer as the earlier benchmark. It distinguishes `Y` from `why`, `cash` from `cache`, and `sea shore` from `seashore`. Aggregate WER counts insertions from control clips. Control rows below use false transcriptions as their denominator.

## Transcription results

The current desktop engine used Rust, WebRTC VAD, and whisper.cpp `small.en` on paced 30 ms frames. The alternative used NVIDIA Parakeet TDT v3 INT8 through sherpa-onnx 2.0.1 on the same WAVs with offline whole-clip decoding. Its decoder saw each complete file. Its result is an acoustic comparison and needs a live streaming test before it can be compared on latency or integrated. [NVIDIA model card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3), [sherpa-onnx model guide](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-transducer/parakeet-tdt.html)

| Condition | Rust Whisper WER | NVIDIA Parakeet offline WER |
| --- | ---: | ---: |
| All 116 clips | 173/960 · 18.02% | 128/960 · 13.33% |
| Normal tongue twisters | 11/176 · 6.25% | 5/176 · 2.84% |
| Fast tongue twisters | 64/320 · 20.00% | 24/320 · 7.50% |
| Quiet tongue twister | 25/56 · 44.64% | 20/56 · 35.71% |
| Homophones | 13/72 · 18.06% | 12/72 · 16.67% |
| Speech in noise | 12/96 · 12.50% | 11/96 · 11.46% |
| Technical words over simulated telephone audio | 30/112 · 26.79% | 45/112 · 40.18% |
| Clipped onset | 8/40 · 20.00% | 7/40 · 17.50% |
| One-word follow-up | 8/8 · 100% | 1/8 · 12.50% |
| Correction across a pause | 0/80 · 0% | 3/80 · 3.75% |
| Non-speech false transcripts | 1/4 | 0/4 |

- Rust emitted text for all 112 speech clips and reported zero frame or final drops. It was exact on 39 speech clips. “Irish wristwatch, Swiss wristwatch” passed all eight voices. “Red lorry, yellow lorry” passed one. The eight “Why?” clips came out as seven `Y` results and one `What?`. The chord control produced `Mm-hmm`. All eight correction clips were exact.
- For the two paired normal/fast phrases, Rust went from 11/176 to 13/176 errors. The larger gap in the fast row also reflects the harder texts added at fast speed. On the 28 clips from the two Indian English synthetic voices, Rust scored 19.58% WER and the NVIDIA candidate scored 15.83%.
- The NVIDIA candidate was exact on 52 speech clips and got seven of eight one-word prompts. Its technical-term recall was 61/104, versus 64/104 for Rust, and its telephone-band WER was worse. Keep the measured live default while testing the candidate with streaming and real microphones. On this run the candidate took 84.27 seconds to decode 537.23 seconds of files after a 2.16-second load.
- Cluely and ParakeetAI are separate commercial services. ParakeetAI documents using Speechmatics for interview transcription. The NVIDIA model result establishes neither product's accuracy. A parity claim requires those products to run the same corpus under comparable settings. [ParakeetAI privacy policy](https://www.parakeet-ai.com/privacy-policy), [Cluely Live Insights](https://docs.cluely.com/feature/liveinsights)

## Python versus Rust performance and memory

Both shipped engines replayed the same 25 clips on the same PC in separate processes. The subset includes three voices, 21 speech clips, four controls, 135.10 seconds, and 216 reference words. The Python baseline was the Tkinter-era `SpeechListener` with faster-whisper 1.2.1, CTranslate2 4.8.5, and CPU/int8 `small.en`. The Rust CLI ran the current desktop ASR path with whisper.cpp `small.en`, CPU, and WebRTC VAD. The Windows memory counters cover each engine process and its model load. The desktop GUI processes are outside this measurement. [Windows memory counters](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-process_memory_counters_ex), [Windows process time](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes)

| Metric | Python | Rust | Observed change |
| --- | ---: | ---: | --- |
| Median final latency on 16 clips recognized by both | 4.694 s | 1.544 s | 67% lower |
| P95 final latency on those same clips | 6.465 s | 2.226 s | 66% lower |
| Median stable preview on 12 clips with one in both | 4.633 s | 2.150 s | 54% lower |
| CPU time including load | 643.84 s | 208.42 s | 68% lower |
| Whole process wall time with paced replay | 200.49 s | 143.24 s | 29% lower |
| Model load | 1.735 s | 0.604 s | 65% lower |
| Peak resident working set | 711.7 MiB | 945.7 MiB | 33% higher |
| Median sampled resident working set | 475.9 MiB | 690.4 MiB | 45% higher |
| Peak private commit | 2,910.3 MiB | 1,308.3 MiB | 55% lower |
| WER | 36/216 · 16.67% | 35/216 · 16.20% | 0.46 percentage points lower |
| Speech clips with no final text | 5/21 | 0/21 | Five recovered |
| Technical-term matches | 19/27 | 17/27 | Two fewer |
| Non-speech false transcripts | 0/4 | 1/4 | One additional |

Resident working set measures physical memory held by the process. Private commit measures memory the OS has committed for it. The Rust build uses more resident memory on this workload while reducing CPU time and private commit. The model formats, inference libraries, VAD, and preview cadence all differ. The data measures the combined pipeline change.

The comparison uses clip hashes from manifest `45e8b4f60facb7ce49549f812c86995d36998478732eaf15eee754948cdb57b9` and 100 ms process sampling. A first Windows probe landed on the virtual environment's launcher process. That memory result was discarded. The Python rerun measured the actual interpreter using the same virtual environment and verified inputs. Absolute manifest paths make this hash specific to this checkout. Each report includes the WAV hashes for cross-checking.

## Reproduce

```powershell
.\.venv\Scripts\python.exe benchmarks/asr.py generate --stress
cargo build --release --locked -p copilot --features audio
.\.venv\Scripts\python.exe benchmarks/asr.py --output benchmarks/output/stress rust --cli target/release/copilot.exe --model models/ggml-small.en.bin
.\.venv\Scripts\python.exe benchmarks/profile_engines.py --cli target/release/copilot.exe --model models/ggml-small.en.bin
```

For the offline reference, use the official `sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2` release asset with SHA-256 `5793d0fd397c5778d2cf2126994d58e9d56b1be7c04d13c7a15bb1b4eafb16bf`. Extract it locally and pass the model directory to `benchmarks/parakeet_probe.py --input benchmarks/output/stress --model <directory>`. `--runtime-dir` can point to an isolated installation of `sherpa-onnx==2.0.1`. [Official release assets](https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models)
