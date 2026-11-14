# Performance bottlenecks

## Decoder work and accuracy

On the 25-clip paired subset, changing the minimum encoder context from 192 to 128 steps cut reported decode from 34.264 to 27.933 seconds and process CPU from 206.719 to 168.812 seconds. It added four word errors and one lost technical-term match. WER rose from 35/216 to 39/216, and stable-preview p95 on the matched 13 clips rose from 4.943 to 5.169 seconds. The smaller floor was rejected. Raw experiment data is in `benchmarks/output/ctx128`.

Loading the model reads and hashes the full verified buffer before native initialization. Peak private commit includes that buffer and decoder state. Both paths retain the integrity check. Pausing releases the audio device and parks the loaded model for 10 minutes, so a resume in that window skips the load and the hash. After 10 minutes, after Stop, or after a failed run, the model is released and the next start loads and verifies it again. The parked model keeps its memory while paused. GUI startup, model readiness and ASR decoding should be profiled separately.

## Live previews

An utterance gets a preview after 1.4 s of speech, and again each time the speech has doubled (at least 2.5 s later), but only while the decoder is idle and the current frame is loud. The loudness check skips the detector's hold of about 270 ms after speech stops, where a preview would start just before the final. All previews of one utterance cost about two greedy passes over it. A Whisper preview is one pass with a token budget, and one above five words a second is dropped as a decoder loop. A Whisper preview gives way once a finished utterance waits for the decoder. Whisper checks after the encoder pass and after each decoded token, so a final can wait for at most one preview encoder pass. Parakeet previews run to completion. Past 15 s of unbroken speech an utterance ends at the first pause of about a third of a second, instead of mid-word at the 24 s cap.

## Streamed answers

Answer text from a provider is appended as it arrives and published by the 50 ms session tick. A fast stream costs at most 20 snapshots a second. Before, every token cloned and serialized the whole session for both windows.

## Desktop metering

Previously a changed audio level cloned and serialized the entire desktop session. Now the audio worker emits a small versioned meter packet. Only mounted, visible meter consumers subscribe. It does not rerender the transcript or answer panels when the level alone changes. Unchanged peaks get a low-rate counter update. The renderer tests reject delayed packets from older sessions or capture runs.

The serialization regression produced **52 bytes per meter update** versus **12,972 bytes for a full snapshot** with the same 80-line fixture. This is a 99.6% payload reduction for that fixture. CPU and UI frame-time effects remain unmeasured.

Run `cargo test --locked --release -p personal-copilot-desktop metering_payload_does_not_reserialize_the_transcript -- --nocapture` with the Windows desktop dependencies and `LIBCLANG_PATH`.

## Interview prep and history

Measured on 13 November 2026 in a release build, i5-1335U. The prep numbers are the desktop command path: copy the workspace, apply one change, compare, save changed rows to SQLite, build the reply.

| Workspace | One prep action p50 | Build reply | Copy workspace | Build pack index | Cold load |
| --- | ---: | ---: | ---: | ---: | ---: |
| 25 questions, 52 answers (three starter packs, resume, job description) | 3.5 ms | 0.35 ms | 0.05 ms | 1.2 ms | 1.7 ms |
| 625 questions, 1,300 answers (the same, 25 times over) | 5.0 ms | 1.6 ms | 0.9 ms | 30 ms | 7.0 ms |

Most of an action is the SQLite commit. Nothing in the prep, practice or community backend was changed. In the Prepare page the open pack now indexes its answers once per data change, and its question list is rebuilt only when the questions, answers or selection change, not on every keystroke in that card.

The saved-session list read each transcript to reach the counts stored after it. A covering index on save time, title and counts fixes that. With 400 saved sessions of 256 lines (a 36 MiB library), timed with the same schema and queries in SQLite directly:

| Query | Before | With the index |
| --- | ---: | ---: |
| List the newest 50 | 6.8 ms | 0.17 ms |
| Search with 50 or more matches | 9.5 ms | 1.3 ms |
| Search with no match | 13.1 ms | 12.9 ms |

A search with no match still scans every transcript. The index is added to an existing library on first open and older builds ignore it.
