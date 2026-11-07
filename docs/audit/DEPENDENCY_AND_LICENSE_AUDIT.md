# Dependency and license audit

OpenKeet declares MIT in [LICENSE](../../LICENSE) and [Cargo.toml](../../Cargo.toml). The benchmark archives and user files are local. [Release readiness](../release-readiness.md) records the distribution and signing gates.

| Scope | Evidence | Review disposition |
| --- | --- | --- |
| Desktop stack | [Cargo.lock](../../Cargo.lock), [package-lock.json](../../apps/desktop/package-lock.json) | Tauri, React, TypeScript, and Vite use permissive upstream licenses. Collect transitives and notices for the packaged build before distribution. |
| Audio | [copilot-audio manifest](../../crates/copilot-audio/Cargo.toml), [Whisper model script](../../scripts/get-speech-model.ps1) | Audit wrapper, vendored C/C++ and models separately. The installer omits speech weights. Downloads are user-initiated and checksum pinned. |
| Legacy Python | [requirements.txt](../../requirements.txt), [start_windows.ps1](../../start_windows.ps1) | Optional packages and separately installed providers belong to the Python runtime. Check third-party and transitive notices if distributing it. |
| Windows runtime | [Tauri config](../../apps/desktop/src-tauri/tauri.conf.json), [installer smoke](../../apps/desktop/tests/installer-smoke.ps1) | NSIS preview uses WebView2. Windows signing and distribution notices are release gates. |
| Local library | [Rust manifest](../../apps/desktop/src-tauri/Cargo.toml), [rusqlite](https://docs.rs/rusqlite/0.41.1/rusqlite/), [SQLite terms](https://www.sqlite.org/copyright.html) | rusqlite and libsqlite3-sys declare MIT. Bundled SQLite is public-domain code. Include the wrapper's notice in a distributable license set. |
| Natively | [License](https://github.com/Natively-AI-assistant/natively-cluely-ai-assistant/blob/7fb88cf56f1e1317075776228615a1f79e8b3c0a/LICENSE), [reference revision](https://github.com/Natively-AI-assistant/natively-cluely-ai-assistant/blob/7fb88cf56f1e1317075776228615a1f79e8b3c0a/README.md) | Personal Use Source License v1.0. Read for feature research. No code, prompts, branding, assets, provider identities, or license text are included in this product. |
| Legacy text bridge | [Local Python adapter](../../free_cluely_bridge.py), [free-cluely reference](https://github.com/Prat011/free-cluely) | The adapter has been in the project since its MIT-licensed first commit. It implements a small loopback HTTP service compatible with the upstream Ollama client. It does not bundle the upstream application or run in the Windows desktop package. |

The [brand guide](../brand/BRAND.md) records the supplied logo artwork, vector trace, and Manrope outlines. The [SIL Open Font License FAQ](https://openfontlicense.org/ofl-faq/) permits logos made with the font. The repository contains artwork paths and no font files. The [screenshot gallery](../community/screenshots.md) uses synthetic questions and test history.

Live model/API credentials and user transcripts must stay outside Git. The Rust desktop preview starts with answers off. Before shipping, verify a secret scan, Rust and npm vulnerability audits, a complete transitive notice list, and installer provenance. The separately maintained Python capture-exclusion code is outside the Rust installer and requires its own privacy review before any distribution.
