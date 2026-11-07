# Windows preview validation

The original private 3.0 preview at `6337948` produced a Windows x64 installer at `target/release/bundle/nsis/Personal Copilot_0.1.0_x64-setup.exe`. The 0.1.0 package version is the preview line. Its SHA-256 was `089065be34b0de2a614d45585d020e4b0a3159774ec4a5a531d85c5b660852df`. That installer was 3,371,412 bytes, versus 2,528,704 bytes for the baseline. The desktop binary was 12,973,568 bytes, versus 10,754,560 bytes. Bundled SQLite adds native code to the desktop build.

## Executed checks

| Command | Observed result |
| --- | --- |
| `cargo fmt --all --check` | Passed. |
| `cargo clippy --locked --release --workspace --all-targets --all-features -- -D warnings` | Passed. |
| `cargo test --locked --release --workspace --all-features -- --nocapture` | 30 tests passed. The dedicated hardware test ran separately. The core suite covers 50 distinct boundary cases plus 78 pause/continuation variants. |
| `cargo test --locked --release -p copilot-audio --test wasapi -- --ignored --nocapture` | Passed against the default and explicit Windows loopback routes with `cache-0.wav`. Both routes produced a nonzero level and recognized “cache” and “Java.” |
| `cargo build --locked --release -p copilot --features audio` and `.\target\release\copilot.exe replay benchmarks/output/interview-replay/from-audio.json` | Passed. Previously captured synthetic ASR text produced one revision-2 answer and one linked follow-up. |
| `npm --prefix apps/desktop test` | Four protocol tests passed. |
| `npm --prefix apps/desktop run desktop -- build --ci -- --locked` | TypeScript, Vite, Tauri release build and NSIS package passed. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File apps/desktop/tests/installer-smoke.ps1` | Temporary installation, native WebView2 tests and uninstall passed. |
| `.\.venv\Scripts\python.exe -B -m unittest discover -s benchmarks -p test_asr.py` | Four tests passed. |
| `npm --prefix apps/desktop audit --omit=dev --json` | Zero vulnerabilities reported. |
| `.\target\audit-tools\bin\cargo-audit.exe audit --json` | Zero vulnerabilities reported. Advisory warnings `RUSTSEC-2024-0370` and `RUSTSEC-2024-0429` remain in the cross-platform lockfile. Neither package appeared in `cargo tree --locked --target x86_64-pc-windows-msvc --prefix none`. |
| `powershell -NoProfile -File benchmarks/desktop-startup.ps1 -Before <baseline-desktop-exe> -After target/release/personal-copilot-desktop.exe` | All 12 rerun samples completed. The initial run reported two handle-close failures. |
| Full speech replay through `benchmarks/asr.py` with the final CLI | All 116 fixtures completed. The prepared manifest matched the previous WAV hashes and order. WER was 18.02%. The first two PowerShell manifest preparation attempts failed before inference. Python's existing JSON helpers rebuilt the manifest. |

Windows Rust checks used `LIBCLANG_PATH` pointing to the retained libclang 18.1.1 directory. The audio test variables pointed to `models/ggml-small.en.bin` and `benchmarks/output/cache-0.wav`. The fixture is synthetic. The build guide has the setup and package commands.

The installed app ran the sample, preserved a checked follow-up, expanded the answer, used the full transcript, and opened the resizable focus window. It saved a session, found it by transcript, reopened notes and deleted the test row. It showed a missing-model error, transcribed real Windows loopback playback, held the automatic answer by default, accepted a manual request, and paused, resumed and stopped. Rust tests exercised the export formats. The smoke confirmed export controls. Native Save-dialog interaction remains unverified.

See the [UI report](../ux/VALIDATION.md) for screenshots and the [performance comparison](../performance/PERFORMANCE_COMPARISON.md) for measured A/B/C data. The [feature matrix](../audit/FEATURE_PARITY_MATRIX.md) tracks implementation gaps.

## Release scope

The original unsigned preview used one selected audio route, local Whisper and a bundled practice responder. The exact legacy Python ZIP, simultaneous dual-route capture, physical microphone and Bluetooth recovery, one-hour sessions, OS scaling across physical monitors, and crash journaling need separate evidence. The CI installer check is available through the opt-in manual workflow input. Checks for this preview ran locally.

## Provider and Windows companion update

The rebuilt x64 installer from `beaf969` is 4,160,796 bytes. Its SHA-256 is `c08efb31344a9ef61f2184e30aa07ec2906da10117b194aa95ba67152df1c6b7`. The temporary install, WebView2 smoke and uninstall passed. The desktop executable is 15,765,504 bytes.

- `cargo fmt --all --check`, release Clippy with warnings denied, 35 workspace Rust tests, four frontend tests, four benchmark Python tests, and the locked Tauri/NSIS build passed.
- Rust tests cover ChatGPT callback state and scope validation, Windows credential protection, cloud consent, Ollama catalog filtering, and fragmented streams with each provider's completion signal.
- The installed WebView2 smoke loads an Ollama catalog from a local HTTP test server. Windows WASAPI captures a synthetic question. **Get answer** sends that question to the server. **Why?** sends the previous question and complete answer in the follow-up request. The answer panel renders list and code blocks from the stream.
- Both native windows retain `WS_EX_TOOLWINDOW` without `WS_EX_APPWINDOW` after pin/unpin and hide/reopen. Shortcut registration succeeded. Windows returned `Access is denied` when the test shell sent keyboard input, so chord-driven foreground activation still needs an unlocked Windows desktop. The same workspace reveal function passed via **Open workspace** from Focus.
- The separately invoked WASAPI test recognized both fixture terms and a nonzero meter on the default and explicit Windows loopback routes. The fixtures and screenshots are local. [UI review and validation](../ux/VALIDATION.md)
- The provider tests exercise local HTTP fixtures. Live sign-in, ChatGPT plan entitlement, Claude billing and answer quality from a real Ollama model remain unverified. [Provider authorization](../audit/PROVIDER_AUTHORIZATION.md)

The optional short follow-up decoder lowered word errors from 55 to 47 on the 60-clip matched pack. The test includes fast short utterances, acknowledgements and controls. The 116-clip stress report continues to describe the standard path. [Audio results and limitations](../benchmarks/interview-stress.md#short-follow-up-mode)

Production npm and RustSec audits reported zero vulnerabilities. RustSec flagged the same two informational advisories in the cross-platform lockfile as in the original preview.
