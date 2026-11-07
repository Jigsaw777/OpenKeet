# OpenKeet launch audit

The launch scope is the Windows 11 x64 preview. The existing GitHub repository is private and has no published packages. The preview is buildable from source. See [release readiness](../release-readiness.md) for the public distribution gates.

## What the preview supports

| Capability | Status | Evidence and limits |
| --- | --- | --- |
| Windows app and NSIS installer | Implemented | [Build and install procedure](../desktop-slice.md#build-and-test). Windows 11 x64 and WebView2. The installer is unsigned. |
| Offline sample | Functional | [Native WebView2 smoke test](../../apps/desktop/tests/native-smoke.mjs) uses built-in synthetic questions and answers, including a follow-up. |
| Live audio | Implemented | [Windows audio capture](../../crates/copilot-audio/src/windows.rs) accepts a microphone or Windows playback endpoint, one at a time. [Validation](../validation/DESKTOP_3_0.md) covers synthetic loopback. Physical mic and headset checks remain open. |
| Local transcription and question detection | Functional with limits | [Speech benchmark](../benchmarks/interview-stress.md) and [session tests](../../crates/copilot-core/tests/session.rs). Short utterances and complex follow-ups can still be missed. |
| Local Ollama and Claude Console | Implemented | [Provider audit](PROVIDER_AUTHORIZATION.md). Local HTTP fixtures verify routing and response parsing. Live Claude account and real-model quality checks remain open. |
| ChatGPT plan usage | Experimental | [Provider audit](PROVIDER_AUTHORIZATION.md) documents the official OAuth flow. Real account eligibility, limits, and inference remain unverified. |
| Local history | Functional | [SQLite tests](../../apps/desktop/src-tauri/src/history.rs) and [native smoke](../../apps/desktop/tests/native-smoke.mjs) cover save, search, notes, and deletion. |
| macOS and Linux desktop | Outside preview scope | The [legacy Python prototype](../legacy-python.md) has source paths for both. The Tauri package is documented for Windows. |

## What remains before going public

- Keep the existing [MIT notice](../../LICENSE). Complete provenance review for source, including the legacy bridge and any adapted snippets. The existing [dependency review](DEPENDENCY_AND_LICENSE_AUDIT.md) records the restricted Natively reference as research only.
- Record public redistribution terms for the maintainer-supplied logo. The gallery uses synthetic app data. No font files, speech weights, or installer binaries are tracked for the launch.
- Produce a dependency bill of materials and the notices for the exact installer, including native speech libraries and installer/runtime components. Confirm the terms for any model or runtime asset that would ship. Whisper weights are downloaded separately in the preview.
- Recheck release history and package contents for credentials, personal material, and unexpected binaries. A pattern scan is one check in that review.
- Verify signing, real provider entitlement, clean Windows installation, and a direct private vulnerability reporting channel before changing visibility or uploading a release.
