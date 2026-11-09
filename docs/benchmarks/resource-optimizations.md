# Memory and speed optimizations (November 2026)

Measured on the same Windows 11 laptop (Intel Core i5-1335U, 16 GB RAM, CPU inference). **Before** is a release build of `main` at `c15e716`. **After** is this change. Runs alternated before and after with the machine otherwise idle.

| What | Before | After | Change |
| --- | ---: | ---: | ---: |
| Speech process peak working set (25 clips, `small.en`) | 948 MiB | 700 MiB | −26% |
| Speech process median working set | 692 MiB | 688 MiB | none |
| Word error rate, profile pack | 16.2% | 16.2% | identical transcripts, 25/25 clips |
| App process tree, hidden to the tray (10 s after hiding) | 397 MiB | 344 MiB | −13% |
| App process tree, visible and idle | 404 MiB | 405 MiB | none |
| Desktop executable size | 20.4 MB | 18.2 MB | −11% |
| Interview prep: open a page (580 questions, 980 answers)¹ | 120 ms | 31 ms | −74% |
| Interview prep: a small edit (same data)¹ | 182 ms | 39 ms | −79% |

¹ Debug build, including the IPC JSON the window receives. Release builds are faster in both columns. Reproduce with `cargo test -p personal-copilot-desktop bench_prep_action_cost -- --ignored --nocapture`.

## What changed

- **Speech model load.** Whisper models were read fully into memory to check their SHA-256, then copied again by Whisper, so a session start briefly held the 465 MB model twice. The file is now hashed through a handle that blocks writes, renames and deletes, and Whisper streams that same file. Paths Whisper's C loader can't open on Windows (non-ASCII) keep the previous verified in-memory copy. Parakeet now holds the same kind of lock until its model is loaded, closing the gap where a file could be swapped after hashing.
- **Hidden windows.** When the main window or overlay is hidden, the app asks WebView2 to lower its memory target and restores it when the window is shown again.
- **Interview-prep store.** Changes are detected by comparing data instead of serializing the whole workspace twice. Only rows that changed are written (upserts), and removed rows are deleted, instead of rewriting every table. The workspace is shared between the store and views instead of copied. Reads skip the copy entirely. Views are serialized once, straight into the IPC response. The bundled community packs are parsed once. The live session's answer index is rebuilt only when the pack in use changes.
- **Release builds** use whole-program optimization (`lto = "fat"`, one codegen unit) and strip symbols. Unwinding stays on because PDF import contains parser panics.

## Tried and not kept

- **Whisper flash attention** cut peak commit by 92 MiB, but changed 19 of 25 transcripts and raised the character error rate from 5.0% to 6.9%, with slightly slower decoding.

Peak private commit of the speech process (about 1.31 GiB) is set by Whisper's decoder buffers, not by model loading, and did not change. The visible app's memory is almost entirely WebView2.

## Reproduce

```powershell
cargo build --release --locked -p copilot --features audio
target\release\copilot.exe bench-asr run --cli <before-or-after copilot.exe> --model models/ggml-small.en.bin --output <folder with the 25-clip profile manifest> --profile
```

The process-tree numbers come from launching each desktop build with a fresh WebView2 profile, waiting 4 seconds, measuring, selecting **Hide**, and measuring again 10 seconds later.
