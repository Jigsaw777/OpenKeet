# OpenKeet launch audit

The launch scope is the Windows 11 x64 preview. The setup index lists unsigned Windows snapshots. [Setup snapshots](../../setup/README.md) · [Public release checklist](../OPEN_SOURCE_RELEASE_CHECKLIST.md) · [Packaging requirements](../release-readiness.md)

## What the preview supports

| Capability | Status | Evidence and limits |
| --- | --- | --- |
| Windows app and NSIS installer | Implemented | [Build and install procedure](../desktop-slice.md#build-and-test). Windows 11 x64 and WebView2. The installer is unsigned. |
| Typed questions | Functional with a connected model | The [native WebView2 smoke test](../../apps/desktop/tests/native-smoke.mjs) uses a local test provider to exercise typed questions and answers. |
| Live audio | Implemented | [Windows audio capture](../../crates/copilot-audio/src/windows.rs) accepts a microphone or Windows playback endpoint, one at a time. [Validation](../validation/DESKTOP_3_0.md) covers synthetic loopback. Physical mic and headset checks remain open. |
| Local transcription and question detection | Functional with limits | Default English Whisper and experimental NVIDIA Parakeet v3. Short utterances and complex follow-ups can still be missed. [Speech benchmark](../benchmarks/interview-stress.md) and [session tests](../../crates/copilot-core/tests/session.rs) |
| Local Ollama and Claude Console | Implemented | [Provider audit](PROVIDER_AUTHORIZATION.md). Local HTTP fixtures verify routing and response parsing. Live Claude account and real-model quality checks remain open. |
| ChatGPT plan usage | Experimental | [Provider audit](PROVIDER_AUTHORIZATION.md) documents the official OAuth flow. Real account eligibility, limits, and inference remain unverified. |
| Local history | Functional | [SQLite tests](../../apps/desktop/src-tauri/src/history.rs) and [native smoke](../../apps/desktop/tests/native-smoke.mjs) cover save, search, notes, and deletion. |
| macOS and Linux desktop | Outside preview scope | The Tauri package targets Windows. Cross-platform prototype code is in the [archived Python revision](../legacy-python.md). |

## What remains before a signed Windows release

- Keep the existing [MIT notice](../../LICENSE) and preserve third-party notices. The [dependency review](DEPENDENCY_AND_LICENSE_AUDIT.md) records the restricted Natively reference as research only and describes the legacy bridge separately.
- Use the maintainer-supplied logo and synthetic app data for the gallery. No font files, speech weights, or installer binaries are tracked for the source preview.
- Produce a dependency bill of materials and the notices for the exact installer, including native speech libraries and installer/runtime components. Confirm the terms for any model or runtime asset that would ship. The preview downloads Whisper and Parakeet model weights separately.
- Recheck release history and package contents for credentials, personal material, and unexpected binaries. A pattern scan is one check in that review.
- Verify signing, real provider entitlement, clean Windows installation, and a direct private vulnerability reporting channel before uploading a release.
