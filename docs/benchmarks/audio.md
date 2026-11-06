# Windows audio comparator

## Reproduce

From the repository root with the legacy dependencies and model caches already installed:

```powershell
.\.venv\Scripts\python.exe -m unittest discover -s benchmarks
.\.venv\Scripts\python.exe benchmarks/asr.py generate
.\.venv\Scripts\python.exe benchmarks/asr.py legacy
```

Generation uses local Windows SAPI and the checked-in recipe. It puts waveforms, references, per-clip hashes, hypotheses, and measured results under `benchmarks/output/`. Keep that directory local. To compare backends, use the same generated manifest. SAPI engine or voice updates can change WAV hashes on regeneration. Legacy replay replaces the hardware `sounddevice` stream with paced 30 ms frames, keeping the production segmentation, mailbox, and decoder. The offline runner requires the three existing model caches.

## Baseline on 6 November 2026

Windows 11 x64 build 26200, i5-1335U, Intel Iris Xe, 16 GiB RAM. Python 3.12.10, faster-whisper 1.2.1, CTranslate2 4.8.5. 32 clips, 163.08 seconds, 252 words, 1,188 normalized characters, and 42 expected technical-term occurrences. Local manifest SHA-256 `ec1153dcd1c7f48637c3fcf76cd23a2991a98a10b9d8678282df312d0eee8d13`. Three US/UK SAPI voices produced the 30 speech clips. Two controls contain silence and a synthetic tone.

| Legacy CPU/int8 | WER | CER | Terms | Missed | False / 2 controls | Stable partial p50 / p95 | Final p50 / p95 | Reported decode RTF |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `small.en` | 32/252 (12.70%) | 11.62% | 36/42 | 5/30 | 0 | 3.586 / 4.469 s | 5.217 / 6.286 s | 1.233 |
| `base.en` | 37/252 (14.68%) | 12.37% | 33/42 | 5/30 | 0 | 1.688 / 2.156 s | 1.507 / 2.090 s | 0.557 |
| `tiny.en` | 41/252 (16.27%) | 13.72% | 29/42 | 5/30 | 0 | 1.352 / 2.953 s | 1.189 / 3.104 s | 0.435 |

- WER sums case-folded word edit distance over all clips, including insertions in the controls. CER removes whitespace and punctuation. Term recall requires a matching word sequence and preserves cash/cache distinctions.
- Stable partial means the first partial prefix of up to two words confirmed by the next update and eventual final for the same utterance. It is measured from clip start, retrospectively. Each model produced 24 eligible clips. Final latency uses the end of the known synthetic speech and the last final event on 25 eligible clips per model. Missing clips are counted separately.
- Reported decode RTF divides the decoder durations on non-empty transcript events by clip seconds. Legacy empty-result timings are unavailable. Wall/process CPU seconds after model load were 251.36/793.88 for `small.en`, 168.36/393.52 for `base.en`, and 173.78/232.45 for `tiny.en`. No drops were reported. This is a single desktop run. Peak memory, power, GPU use, and driver queue lag require separate measurement.
- All three models missed the same three below-threshold WASAPI questions and two short “Why?” clips. The WASAPI label here is the spoken term. This replay never opened a hardware endpoint. `small.en` recognized QNX in all three voices and corrupted “coroutine” once and “cash flow” once. The noisy and correction clips retain their independent scores in each JSON report.

The data shows that changing model size alone cannot recover clips rejected before ASR. The new path needs to improve quiet and short speech while retaining term accuracy and silence rejection. This sample supports comparison under controlled inputs. Indian English, human speakers, physical headsets, and real meeting codecs remain open gates.

## Rust and Windows checks

Run the release CLI with the `audio` feature and compare the original manifest. The Rust runner opens each WAV on its own paced clock, uses the same bounded pipeline as Windows capture, and reuses the model across clips.

```powershell
cargo build --release --locked -p copilot --features audio
.\.venv\Scripts\python.exe benchmarks/asr.py rust --cli target/release/copilot.exe --model models/ggml-small.en.bin
.\.venv\Scripts\python.exe benchmarks/asr.py rust --cli target/release/copilot.exe --model models/ggml-large-v3-turbo-q5_0.bin
```

| Rust CPU path | WER | CER | Terms | Missed | False / 2 controls | Stable partial p50 / p95 | Final p50 / p95 | Reported decode RTF |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `small.en` | 19/252 (7.54%) | 1.85% | 35/42 | 0/30 | 0 | 2.537 / 2.624 s | 1.768 / 2.100 s | 0.427 |
| `small.en` with hints | 15/252 (5.95%) | 1.77% | 38/42 | 0/30 | 1 | 2.968 / 17.776 s | 2.450 / 17.404 s | 0.997 |
| `large-v3-turbo` Q5 | 73/252 (28.97%) | 18.94% | 34/42 | 0/30 | 1 | 5.038 / 9.529 s | 6.799 / 45.093 s | 2.924 |

- Both use WebRTC VAD, one optional rolling preview per utterance, up to six CPU threads, no GPU, and an encoder context sized to the segment. The files, normalization, and reference counts match the Python rows. Each produced a stable partial on 26 clips and a final on 30 speech clips. The small model had a maximum 0.266 s decoder queue wait. Turbo reached 33.138 s. The desktop build overlapped part of the turbo run, so treat its latency figures as a development-host observation.
- Keep `small.en` as the starting point on this PC. Compared with the Python `small.en` baseline, this run cut WER by 40.6% relative and median final latency by 3.45 s. The two implementations differ in VAD, preview scheduling, decoding and vocabulary, so the gain cannot be attributed to Rust alone. Both controls produced empty final text or no transcript with the Rust small model.
- Rust `small.en` returned surrounding words in all three quiet clips but missed the name “WASAPI” in each. All three one-word “Why?” clips decoded as “Y.” It preserved “cash flow” on all three voices. Turbo repeated phrases in several clips and hallucinated a sentence on the tone control. A non-empty result does not imply a correct transcript.
- The optional hints were `Java, Kotlin, Android, QNX, WASAPI, PostgreSQL, coroutine, idempotency keys, mutex, semaphore, LRU cache, cash flow`. They recovered three more term occurrences and kept cash/cache correct on the corresponding clips. WASAPI still failed as a term, and the tone produced “and.” The run overlapped packaging work and had one additional stable-partial clip. Leave hints opt-in and repeat latency measurements under controlled load before tuning them as a preset.

The Windows hardware smoke uses only a generated clip. Stop unrelated audio first. It refuses to run if the default output has an active audio session. The test plays the fixture through the OS and verifies a nonzero meter and the words “cache” and “Java” in a final transcript for the default and explicitly selected endpoint.

```powershell
$env:COPILOT_TEST_MODEL = (Resolve-Path models/ggml-small.en.bin).Path
$env:COPILOT_TEST_WAV = (Resolve-Path benchmarks/output/cache-0.wav).Path
cargo test --release --locked -p copilot-audio --test wasapi -- --ignored --nocapture
```

Both routes passed on Speaker (Realtek(R) Audio) after the short-gap recovery change. The first probe reported a 2 ms driver discontinuity and stopped. Capture now reports and fills measurable gaps of up to 30 ms with silence. Larger or unmeasurable gaps require restart. The hardware smoke checks one clip. Use the file matrix to compare WER and latency across all 32 clips.
