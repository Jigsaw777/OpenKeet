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
