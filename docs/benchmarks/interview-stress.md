# Interview speech stress test

## Corpus and scoring

Run on Windows 11 x64 with an Intel i5-1335U and 16 GiB RAM. The pack contains 112 spoken clips and four non-speech controls, with 537.23 seconds of audio and 960 reference words. Eight offline Windows voices cover US, UK, and Indian English locales. This measures synthetic speech. Human speakers, conferencing applications, and mixed-room recordings need separate tests. [Windows speech synthesis](https://learn.microsoft.com/en-us/uwp/api/windows.media.speechsynthesis.speechsynthesizer)

The recipe covers tongue twisters at normal and 1.65× synthesis speed, homophones, a quiet peak of 0.008, 8 dB added noise, a simulated 8 kHz telephone path, an 80 ms clipped onset, short “Why?” prompts, and corrections across a 1.6-second pause. Controls include silence, a tone, synthetic keyboard clicks, and synthesized chords. All waveforms stay in the ignored output folder. The run used manifest SHA-256 `b79cb51873988fd561a441ed6b00cc4a0dd707c0e4c5125f7f888dee14fc3893`.

WER uses the same case-folded, punctuation-insensitive scorer as the earlier benchmark. It distinguishes `Y` from `why`, `cash` from `cache`, and `sea shore` from `seashore`. Aggregate WER counts insertions from control clips. Control rows below use false transcriptions as their denominator.

## Transcription results

The current desktop engine used Rust, WebRTC VAD, and whisper.cpp `small.en` on paced 30 ms frames. The alternative used NVIDIA Parakeet TDT v3 INT8 through sherpa-onnx 2.0.1 on the same WAVs with offline whole-clip decoding. Its decoder saw each complete file. Its result is an acoustic comparison. The [live pipeline comparison](#parakeet-in-the-live-pipeline) below measures latency and CPU. [NVIDIA model card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3), [sherpa-onnx model guide](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-transducer/parakeet-tdt.html)

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

## Parakeet in the live pipeline

The desktop app now offers NVIDIA Parakeet TDT 0.6B v3 INT8 as an optional engine under Settings → Audio and display. It runs through sherpa-onnx 2.0.1 on the CPU with six threads. Both engines below used the same release CLI, the same WebRTC VAD segmentation, 30 ms paced replay, one preview per utterance, and the 116 clips from the manifest above. The runs were sequential on 7 November 2026, Whisper from 3:54 pm IST and Parakeet from 4:21 pm IST, with no build running. The Parakeet files are the pinned export described under Reproduce. Process counters include model load. [Per-clip results for both engines](../../benchmarks/results/stress-engines.json)

| Metric | Whisper `small.en` | Parakeet v3 INT8 |
| --- | ---: | ---: |
| WER | 173/960 · 18.02% | 128/960 · 13.33% |
| CER | 354/4,576 · 7.74% | 209/4,576 · 4.57% |
| Exact speech clips | 39/112 | 48/112 |
| Technical-term matches | 64/104 | 62/104 |
| Final latency p50 / p95, same 112 clips | 1.353 / 3.011 s | 1.090 / 1.582 s |
| Stable preview p50 / p95 | 2.183 / 4.775 s on 86 clips | 1.900 / 2.008 s on 92 clips |
| Reported decode time | 179.7 s | 81.3 s |
| Process CPU time | 1,013.1 s | 289.8 s |
| Model load, including SHA-256 checks | 0.683 s | 2.270 s |
| Peak / median working set | 947.7 / 692.7 MiB | 800.3 / 792.5 MiB |
| Peak private commit | 1,312.6 MiB | 831.6 MiB |
| Largest decoder queue wait | 2.218 s | 0.001 s |
| Non-speech false transcripts | 1/4 | 1/4 |

| Condition | Whisper live | Parakeet live | Parakeet offline |
| --- | ---: | ---: | ---: |
| Normal tongue twisters | 11/176 · 6.25% | 5/176 · 2.84% | 5/176 · 2.84% |
| Fast tongue twisters | 64/320 · 20.00% | 25/320 · 7.81% | 24/320 · 7.50% |
| Quiet tongue twister | 25/56 · 44.64% | 16/56 · 28.57% | 20/56 · 35.71% |
| Homophones | 13/72 · 18.06% | 11/72 · 15.28% | 12/72 · 16.67% |
| Speech in noise | 12/96 · 12.50% | 11/96 · 11.46% | 11/96 · 11.46% |
| Technical words over simulated telephone audio | 30/112 · 26.79% | 45/112 · 40.18% | 45/112 · 40.18% |
| Clipped onset | 8/40 · 20.00% | 7/40 · 17.50% | 7/40 · 17.50% |
| One-word follow-up | 8/8 · 100% | 1/8 · 12.50% | 1/8 · 12.50% |
| Correction across a pause | 0/80 · 0% | 6/80 · 7.50% | 3/80 · 3.75% |

- Parakeet produced its final sooner on all 112 paired speech clips. Seven of eight “Why?” clips came out as `Why?`. The two Indian English voices scored 17.08% WER, versus 19.58% for Whisper. Both engines produced one false transcript, on the chord control.
- Parakeet regressed on technical words over telephone-band audio and on corrections across a pause. Three of eight correction clips missed words: “semaphore” became “Cemophore” and “a CMA for”, and “I meant a” became “I'm into”. VAD split every correction clip at the pause, so each half was decoded alone. Whisper's error, term and false-transcript counts matched its earlier run.
- The live and offline Parakeet runs had the same 128 word errors with different text on 41 clips. Segment boundaries and trimmed trailing silence changed individual words in both directions.
- ONNX Runtime threads spin while idle by default. With spinning, the same Parakeet run used 2,284.1 s of CPU time with identical transcripts. The engine now disables intra-op and inter-op spinning. On a 12-clip Python probe that cut CPU time from 128.9 s to 25.5 s and added 1.4% to decode time.
- The live WASAPI smoke played `cache-0.wav` on the default Speaker (Realtek(R) Audio) output and captured it through loopback. Both engines returned “when would you use an lru cache in java?” on the default and explicit routes. Whisper's final arrived 1,334 and 1,350 ms after the last voiced frame. Parakeet's arrived 899 and 870 ms after it. The first Parakeet attempt stopped before the explicit route because the quiet-output guard saw another audio session. The repeat passed both routes.
- The desktop native smoke passed with each engine, with the Windows shell and provider fixture checks on. It played the same clip through system output and waited for “Question heard” and a final line containing cache and Java. A clean-profile Parakeet download from the app took 57.1 s. All four files matched their pinned sizes and SHA-256 values, and no staged file remained.
- On this PC, this measurement shows Parakeet ahead on WER and final latency. The default stays `small.en` in this change because of the telephone and correction regressions and the synthetic-only corpus. Human speakers, conferencing codecs and real microphones remain untested. Parakeet is also a 670 MB download, versus 488 MB for Whisper.

## Short follow-up mode

The 60-clip follow-up pack pairs 48 new fast clips from eight Windows voices with eight original “Why?” clips and the four stress controls. All variants used the same PCM hashes and pinned `small.en` model. **Short follow-up mode** runs one greedy decode for finals whose buffered PCM is at most 1.25 seconds. Longer finals use the original beam search. The option is off at session setup.

| Metric | Standard beam | Short follow-up mode | Rejected second pass with hints |
| --- | ---: | ---: | ---: |
| Word errors and WER | 55/88 · 62.50% | 47/88 · 53.41% | 310/88 · 352.27% |
| Character errors | 162/312 | 140/312 | 1,386/312 |
| Final p95, 56 spoken clips | 2.946 s | 2.547 s | 15.286 s |
| Reported decode time | 93.586 s | 78.740 s | 367.480 s |
| Empty speech results | 0/56 | 0/56 | 0/56 |
| False transcripts on controls | 1/4 | 1/4 | 1/4 |

“And then?” improved from 13/16 to 7/16 word errors. “Why?” remained at 7/8 errors. “What changed?” and “Which one?” had equal total errors with different individual failures. These are single-process synthetic runs on the same PC with background build activity. Human short speech and codec performance need separate validation. [Per-clip results and fixture hashes](../../benchmarks/results/short-followups.json)

The initial hinted second pass on short or quiet speech scored 166/960 word errors on all 116 stress clips, versus 173/960 for the standard decoder. Character errors rose from 354 to 367 and final p95 rose from 2.678 s to 6.017 s. The second pass repeated phrases in the varied short pack. Insertions count toward WER, which can exceed 100%. The shipped short mode uses a single pass with no added prompt.

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
cargo build --release --locked -p copilot --features audio
target\release\copilot.exe bench-asr generate --stress
target\release\copilot.exe bench-asr run --output benchmarks/output/stress --cli target/release/copilot.exe --model models/ggml-small.en.bin
target\release\copilot.exe bench-asr profile --cli target/release/copilot.exe --model models/ggml-small.en.bin
```

For the live pipeline comparison, add `--profile` to record process CPU and memory. A model folder selects Parakeet in the CLI. The app downloads that folder to `%LOCALAPPDATA%\dev.personalcopilot.desktop\models`.

```powershell
target\release\copilot.exe bench-asr run --output benchmarks/output/stress --cli target/release/copilot.exe --model models/ggml-small.en.bin --profile
target\release\copilot.exe bench-asr run --output benchmarks/output/stress --cli target/release/copilot.exe --model models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8 --profile
```

The WASAPI smoke and `npm run test:native` accept the same folder in `COPILOT_TEST_MODEL`. Windows 11 ships an older `onnxruntime.dll` in System32, and the loader searches there before `PATH`. Copy the two bundled DLLs next to the test binary first. The app, the CLI and the installer already keep them beside the executable.

```powershell
Copy-Item target/release/onnxruntime.dll, target/release/sherpa-onnx-c-api.dll target/release/deps/
$env:COPILOT_TEST_MODEL = (Resolve-Path models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8).Path
$env:COPILOT_TEST_WAV = (Resolve-Path benchmarks/output/cache-0.wav).Path
cargo test --release --locked -p copilot-audio --test wasapi -- --ignored --nocapture
```

For the offline reference, run `target\release\copilot.exe bench-asr parakeet --input benchmarks/output/stress --model <directory>`. It decodes each whole clip with the app's own sherpa-onnx 2.0.1 runtime. The first offline run used the `sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2` release asset with SHA-256 `5793d0fd397c5778d2cf2126994d58e9d56b1be7c04d13c7a15bb1b4eafb16bf`. Upstream replaced that asset and the Hugging Face files with a new export on 7 November 2026 at 11:20 am IST. The replacement archive has SHA-256 `1495e91b59d7fc53ddca91f3431758ec99f8fa253fdb0e1e851be0f72b14b0f9` and was not measured. The desktop download and these runs use the earlier export, pinned to Hugging Face revision `2bda32ec70b097a55adaa07d9a7173915b43cc78`. Its four files reproduce all 116 earlier offline transcripts exactly. [Official release assets](https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models), [Pinned export](https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/tree/2bda32ec70b097a55adaa07d9a7173915b43cc78)
