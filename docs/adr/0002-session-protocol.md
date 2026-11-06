# 0002 · Session protocol v1

The CLI and desktop use the same `copilot-core` reducer. Event envelopes carry a protocol version, monotonic sequence, session ID, source, capture span, and optional question ID and revision. PCM and credentials stay outside the protocol. Consumers can initialize from a snapshot and reject updates from an older session or sequence.

```text
audio activity → interim/stable transcript → final utterance
                                     ↓
                    new / continue / follow-up / ignore
                                     ↓
                   candidate / incomplete / waiting
                                     ↓
                      draft or checked request
                                     ↓
               checked history / superseded / error
```

Finals arrive in capture order from the local worker. Interim snapshots replace text by utterance ID and revision. An unresolved capture span holds a checked answer until its final clarifies whether it belongs to that question. A continuation within five seconds increments the revision and cancels old work. Explicit corrections can revise within 15 seconds. A scenario without an actual question stays incomplete through the pause. An acknowledgement stays in the transcript and never starts an answer.

A contextual follow-up within two minutes owns a new question ID and history entry. Its request carries the prior question and its matching checked answer when available. A stale draft is never forwarded as checked context. The new fields are optional additions to protocol v1. Checked completions are matched by `(session, question, revision, mode)` and can update an existing history entry only after validation. Timeouts and cancellation transition out of busy state.

The v1 classifier uses local boundary cues. The CLI fixture responder shows protocol behavior with canned practice answers. Provider-side semantic classification and long-stream word stability are separate gates. Pending work, transcripts, history, and retained events have fixed limits. An audio gap invalidates uncertain work and closes the affected question boundary. The full-transcript action reads finalized text and reports when the in-memory limit prevents a complete copy.

Run `cargo run -p copilot -- replay` from the workspace. The final NDJSON record is the snapshot. The script includes a 1.6-second pause, an incorrect interim, a late constraint, and a separate “what if” follow-up. `cargo test -p copilot-core` exercises the same reducer without sound hardware or a provider account.
