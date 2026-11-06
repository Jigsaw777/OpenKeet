# 0001 · Rust engine and Tauri desktop

Status: accepted for the vertical slice.

The Tkinter app has working capture, manual and automatic answers, and context import. Its UI and session lifecycle share mutable state. Keeping Tk as the product surface would leave the same event and view coupling in place while replacing the engine. Use React/TypeScript for the new desktop surface and Tauri 2 for windowing and IPC. Keep the Python app runnable during migration. [Tauri architecture](https://v2.tauri.app/concept/architecture/)

```text
Legacy launchers ──> Python UI, audio, question state, providers
                                │
                       Comparator + golden fixtures
                                │
Windows WASAPI or WAV replay ──> bounded, source-tagged PCM frames
                                │
                         VAD + local STT worker
                                │
                        Rust question reducer ───> bounded answer work
                                │                      │
                  versioned events and snapshots <─────┘
                        │                    │
                    Rust CLI         Tauri IPC ──> React views
```

Start with one domain crate and separate adapter/shell boundaries. Cargo features keep audio inference out of the portable domain build. Render typed session state in the UI and check `(session_id, question_id, revision)` before every answer update and commit. Use monotonic milliseconds since the session started. System output, microphone, and replay are route identities. A route carries device identity, not a speaker identity. Capture never waits for downstream decode. Saturation ends the affected stream with a visible error or records a bounded discontinuity before recovery.

Rust enables a reusable reducer and explicit ownership of buffers, workers, and cancellation. Recognition accuracy depends on input quality, VAD, model, decoding, and language coverage. Benchmark those separately before changing model defaults.

Replay and a fixture responder are the first verification gate. Next connect explicit local Windows capture and a native recognition worker through the same protocol. Keep network calls and durable storage outside the reducer. A future networked provider receives a validated request and returns a typed classification and stream, with no shell/file access and no tokens in IPC.
