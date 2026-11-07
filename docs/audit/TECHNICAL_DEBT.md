# Baseline technical debt

| Priority | Finding | Verification and next check |
| --- | --- | --- |
| P0 | [workspace.rs](../../apps/desktop/src-tauri/src/workspace.rs#L270) accepts a delayed Ready event after Pause or Stop. A second Pause can overwrite the pending Stop. | Reproduce both command/event orderings with the real desktop state machine. Keep draining transcript events before publishing the terminal state. |
| P0 | [pipeline.rs](../../crates/copilot-audio/src/pipeline.rs#L177) drops active speech when the capture producer fails. [windows.rs](../../crates/copilot-audio/src/windows.rs#L191) treats device loss as fatal. | Feed valid PCM followed by a simulated input failure. Ensure an already captured tail can be drained and the error remains visible. |
| P0 | [begin_audio](../../apps/desktop/src-tauri/src/workspace.rs#L151) accepts a single route and rebuilds inference on resume. | Verify baseline against fixed Realtek loopback WAV. Dual routes, switch/reconnect, physical microphone and Bluetooth require independent route identity and hardware testing. |
| P0 | [ready](../../crates/copilot-core/src/session.rs#L634) uses a fixed silence deadline. Live audio starts the rehearsal responder without a separate auto-answer setting. | Preserve draft revision and follow-up tests. Add an explicit per-session opt-in and manual commit. |
| P0 | [workspace.rs](../../apps/desktop/src-tauri/src/workspace.rs#L67) stores one in-memory session. | Verify save/load across process restarts and delete through application entry points. |
| P1 | [emit](../../apps/desktop/src-tauri/src/workspace.rs#L332) serializes the whole transcript and answer history for meter changes. | Measure payload work separately from inference. Batch or split metering without regressing renderer ordering. |
| P1 | [LocalWhisper::load](../../crates/copilot-audio/src/whisper.rs#L37) reads the pinned weight file into a full buffer before allocation of Whisper context. | Retain byte-level integrity. Profile load and peak commit before changing memory ownership. |
| P1 | [main.tsx](../../apps/desktop/src/main.tsx#L23) owns app navigation, state and workspace. | Keep the verified answer reading area and compact window. Separate high-traffic views and centralize theme/spacing before new screens. |
| P1 | [README](../../README.md) describes legacy auth/provider flows. Rust `rehearsal` (since removed) was offline practice. | Report these separately. Add a tested local provider before enabling other traffic. |
| P2 | [preview.yml](../../.github/workflows/preview.yml) is manually dispatched. | Add reproducible checks without dispatching paid or hosted runs. |
