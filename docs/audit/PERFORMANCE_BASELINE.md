# Performance baseline

Baseline A uses the Git-based Python fallback documented in [legacy baseline](LEGACY_PYTHON_BASELINE.md). Baseline B uses a fresh release CLI from the byte-matched [7z source](CURRENT_ARCHITECTURE.md). Both run on Windows build 26200 with Intel Core i5-1335U, Iris Xe, 15.72 GiB visible RAM, Balanced power mode, and CPU inference.

Run `target\release\copilot.exe bench-asr profile --cli <frozen-cli> --model models/ggml-small.en.bin --output benchmarks/output/baseline-3.0/profile`. (The Python half of this baseline was measured with the former Python harness and is frozen in `benchmarks/results/python-baseline.json`.) It selects 25 clips, including four controls, from the 116-clip offline synthetic corpus. The runner pairs each clip ID and WAV SHA-256. Full stress evidence, known model trade-offs, and the earlier measurement are in [interview stress](../benchmarks/interview-stress.md).

The comparison is a **product pipeline** comparison. Faster-Whisper INT8 and whisper.cpp use differently formatted weights and different VAD/decoding. The Rust before/after replay can hold model, waveform, and backend fixed. A language-only comparison would require the same inference backend and processing policies.

This lane measures paced ASR, CPU, process memory and transcript metrics. GUI startup and memory, GPU, one-hour drift, hardware switching, human speech, real LLM timing, and hosted recognition are outside the lane. Those metrics require separate runs. Keep raw JSON and Windows profile counters under the ignored output directory.

## Baseline run

The new run completed with 135.097 seconds of WAVs, 21 speech clips, four controls, and 216 reference words. Manifest SHA-256 is `45e8b4f60facb7ce49549f812c86995d36998478732eaf15eee754948cdb57b9`. Memory sampling is 100 ms. Each result is from one sequential process run. No cold OS cache reset was performed.

| Metric | Python A | Rust B |
| --- | ---: | ---: |
| WER | 36/216 (16.67%) | 35/216 (16.20%) |
| CER | 131/1,020 (12.84%) | 51/1,020 (5.00%) |
| Final latency p50, 16 common clips | 4.920 s | 1.546 s |
| Final latency p95, same 16 clips | 6.308 s | 2.224 s |
| CPU time | 631.703 s | 206.719 s |
| Process wall time | 197.844 s | 143.719 s |
| Peak working set | 711.9 MiB | 947.7 MiB |
| Peak private commit | 2,907.3 MiB | 1,306.7 MiB |
| Missed speech clips | 5/21 | 0/21 |
| False transcripts | 0/4 | 1/4 |

Raw measurements and transcripts are under `benchmarks/output/baseline-3.0/profile/`. The legacy environment had Faster-Whisper 1.2.1 and CTranslate2 4.8.5. The Rust release uses whisper-rs 0.16.0 and the pinned `ggml-small.en.bin` with SHA-256 `c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d`.

`npm ci --no-audit --no-fund`, `npm test` (3 tests), `npm run build`, and `cargo build --locked --release -p copilot --features audio` passed. `npm --prefix apps/desktop run test:native` passed on the retained B desktop executable with the bundled sample and missing-model recovery. The baseline screenshot folder is `benchmarks/output/baseline-3.0/ui/`. Split answer width was 501 CSS px, expanded width was 863 CSS px, and the compact reading area used 273 CSS px of a 480 CSS px viewport.
