# Performance bottlenecks

## Decoder work and accuracy

On the 25-clip paired subset, changing the minimum encoder context from 192 to 128 steps cut reported decode from 34.264 to 27.933 seconds and process CPU from 206.719 to 168.812 seconds. It added four word errors and one lost technical-term match. WER rose from 35/216 to 39/216, and stable-preview p95 on the matched 13 clips rose from 4.943 to 5.169 seconds. The smaller floor was rejected. Raw experiment data is in `benchmarks/output/ctx128`.

Loading the model reads and hashes the full verified buffer before native initialization. Peak private commit includes that buffer and decoder state. Both paths retain the integrity check. Pausing currently releases inference and audio device resources. Resuming reloads the model. GUI startup, model readiness and ASR decoding should be profiled separately.

## Desktop metering

Previously a changed audio level cloned and serialized the entire desktop session. Now the audio worker emits a small versioned meter packet. Only mounted, visible meter consumers subscribe. It does not rerender the transcript or answer panels when the level alone changes. Unchanged peaks get a low-rate counter update. The renderer tests reject delayed packets from older sessions or capture runs.

The serialization regression produced **52 bytes per meter update** versus **12,972 bytes for a full snapshot** with the same 80-line fixture. This is a 99.6% payload reduction for that fixture. CPU and UI frame-time effects remain unmeasured.

Run `cargo test --locked --release -p personal-copilot-desktop metering_payload_does_not_reserialize_the_transcript -- --nocapture` with the Windows desktop dependencies and `LIBCLANG_PATH`.
