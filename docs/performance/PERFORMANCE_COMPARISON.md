# OpenKeet 3.0 performance comparison

See the [generated A/B/C table](../../benchmarks/results/comparison.md) and [CSV](../../benchmarks/results/comparison.csv). The [benchmark instructions](../../benchmarks/README.md) reproduce the work. Windows 11 build 26200, Intel i5-1335U, Iris Xe, 15.72 GiB visible RAM, Balanced, CPU inference. All 25 WAVs match by SHA-256. They contain 21 spoken synthetic clips and four controls, 135.097 seconds total and 216 reference words.

| Run | Source | SHA-256 or identity |
| --- | --- | --- |
| A | Unchanged pre-migration Python ASR, Faster-Whisper 1.2.1, CTranslate2 4.8.5 CPU/INT8 `small.en` | Git tree `04bfadf973cb30880fdd487cb3bbef9645416e73`. The original ZIP is unavailable. |
| B | Release CLI built from byte-matched 7z source `d2d8069` | Executable `ffa3ba57712a941057b60fcb0891ba4467ecdabf7e9b3abe983c5aa8d7b6ab82` |
| C | Release CLI at `6722ec2` with the same decode options used in the resulting desktop preview | Executable `f45a43c9fb5185b96cb09b6a8df44e35ded3cffee3e81b101063cf9a40200f8e` |

B and C use the same CPU Whisper backend, pinned `small.en` weights, and VAD parameters. A and C differ in inference library, model format and endpointing, so their deltas describe the combined product pipelines. All C samples used the restored 192-step decoder floor.

| Metric | A | B | C first | C repeat |
| --- | ---: | ---: | ---: | ---: |
| Final p95, 16 common clips | 6.308 s | 2.224 s | 2.232 s | 2.228 s |
| CPU time | 631.703 s | 206.719 s | 204.656 s | 203.312 s |
| Peak working set | 711.9 MiB | 947.7 MiB | 947.7 MiB | 945.7 MiB |
| Peak private commit | 2,907.3 MiB | 1,306.7 MiB | 1,306.8 MiB | 1,308.6 MiB |
| WER | 36/216 | 35/216 | 35/216 | 35/216 |
| Technical term matches | 19/27 | 17/27 | 17/27 | 17/27 |
| Non-speech controls with false text | 0/4 | 1/4 | 1/4 | 1/4 |

C used about 68% less CPU and 55% less private commit than A while using 33% more resident memory. All B and C results produced final text for each spoken clip. Neither C run shows a material ASR gain over B. The 100 ms process memory samples include model load and exclude GUI/WebView processes. Small samples and file-cache effects limit latency inference. A and B have one fresh process run each. C has two. Development shell activity and a brief frontend build occurred during the paired measurements. There were no timed-out runs in the reported A/B/C table.

## Full 116-clip transcription check

The final release CLI at `0ca6b39` replayed all 116 stress clips with the pinned Whisper model. Clip IDs, hashes, and recognized text match the [earlier 116-clip report](../benchmarks/interview-stress.md) for every clip. Aggregate WER was **173/960 (18.02%)**, technical-term recall was **64/104**, and all 112 spoken clips produced final text. Final p95 was **2.678 s** across the 112 spoken clips. The run reported zero frame and final drops and one false transcript in four controls. The run covered 537.228 seconds of audio and completed in 561.281 seconds with load. [Full per-clip results](../../benchmarks/results/stress-current.json).

Normal tongue twisters had 11/176 errors and fast tongue twisters had 64/320. The quiet condition had 25/56 errors. The one-word prompts missed the intended word on all eight clips. Corrections across a pause were exact on all eight clips. The holdout requirement still needs human speech, consented device recordings and mixed audio. The previous Parakeet result used an offline decoder and remains a separate acoustic comparison.

| Other metrics | Status and reproduction |
| --- | --- |
| GUI launch and idle memory tree | See the separate Windows desktop probe below. Interactive startup, active tree CPU/RAM and GPU remain unmeasured. |
| End-to-end answer time, time to first token and rate limits | Unmeasured. The original preview used the practice responder. The new provider protocol and local loopback path have fixture coverage. |
| Audio ready, decode, queue and loss | Available in Audio Setup during capture. Run the [WASAPI integration test](../../crates/copilot-audio/tests/wasapi.rs) for synthetic playback on the two loopback selection paths. |
| Queue saturation and failure tail | Unit and integration regressions cover explicit loss and valid queued PCM after failure. |
| One-hour drift, overlapping speakers, Bluetooth, hardware reconnect | N/A. Requires consented fixtures, devices and long-session instrumentation. |
| UI navigation and long tasks | Payload sizing and native layout tests are separate. No measured frame-rate or render-time claim. |

Raw C repeat reports are local under `benchmarks/output/optimized-3.0/repeat`. [The initial paired reports](../../benchmarks/results) exclude host filesystem paths.

## Windows desktop startup and idle memory

The [desktop probe](../../benchmarks/desktop-startup.ps1) launched B and C three times each with fresh WebView2 profiles and three more times reusing the first profile. Both conditions start a fresh process. The OS file cache was uncontrolled. Window time is the elapsed time until a visible native handle appears, sampled every 20 ms. Idle memory is the sum of the application and its descendant processes' counters after the window and a 750 ms settle. Working-set sums can count shared pages more than once.

| Metric, median of three | B fresh | C fresh | B reused | C reused |
| --- | ---: | ---: | ---: | ---: |
| First native window | 45 ms | 35 ms | 40 ms | 46 ms |
| Observed range | 41–73 ms | 35–47 ms | 35–44 ms | 35–47 ms |
| Summed working set | 366.8 MiB | 357.4 MiB | 362.9 MiB | 352.8 MiB |
| Summed private memory | 176.0 MiB | 166.8 MiB | 172.7 MiB | 162.8 MiB |
| App close to process exit | 46 ms | 49 ms | 46 ms | 47 ms |

The launch ranges overlap. C's sampled idle tree used about 9–10 MiB less resident and private memory than B in both conditions. A's GUI startup and idle tree are unmeasured. [Rerun samples](../../benchmarks/results/desktop-startup.json).

The [initial probe](../../benchmarks/results/desktop-startup-initial.json) logged one close failure for each build. The probe refreshed the native handle before retrying close. All 12 runs completed in the rerun. The initial B launch of 707 ms illustrates the uncontrolled cache effects.
