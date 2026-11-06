# Personal Copilot 3.0 Windows preview

## Run the preview

1. Run the Windows build and choose **Run sample session**. It replays a question with a 1.6-second pause and a late constraint. The Rust engine checks the consolidated question, then uses it as context for a separate “what if” follow-up. The answers come from a bundled practice responder.
2. For live local transcription, get a verified speech model and open **Audio setup**. Choose an output or microphone, select the model file, confirm capture permission, and select **Start local capture**. Play a known fixture or speak into the selected source. Confirm the meter and transcript. Use **Suggest answer** on a detected question, or explicitly enable automatic suggestions in Audio Setup.
3. Use **Pause**, **Resume**, or **Stop** to control capture. **Retry same source** resumes a recoverable device failure in the current session. **Use full transcript** puts finalized text in the question field. **Answer view** uses the workspace width for the current answer. **Focus window** opens a normal, pinnable window with a scrollable answer, an expandable question, and a fixed copy button.
4. After stopping, choose **Save to history** to retain the available final transcript and checked answers in SQLite. Find it under **Saved sessions** to add notes, export it, or delete it. **Clear current session** clears the workspace.

The preview uses local Whisper and bundled example responses. Audio stays in memory. Context import, general model answers, and provider sign-in remain open in Rust. The existing Python launchers provide the legacy provider and profile workflows.

## Build and test

Install stable Rust with the `x86_64-pc-windows-msvc` target, VS C++ build tools and Windows SDK, CMake, LLVM/libclang, and Node 24. Windows 11 supplies WebView2 on supported installs. A Visual Studio developer shell can expose CMake. Set `LIBCLANG_PATH` to the directory containing the MSVC-compatible `libclang.dll` when the LLVM bin directory is outside `PATH`. The Windows Whisper build generates its bindings against the native headers. [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), [Bindgen requirements](https://rust-lang.github.io/rust-bindgen/requirements.html)

```powershell
cargo test --locked -p copilot-core -p copilot-audio -p copilot --features copilot/audio
cargo run --locked -p copilot -- replay
Set-Location apps/desktop
npm ci
npm test
npm run desktop -- build --ci -- --locked
npm run test:native
# Verify the built installer when Personal Copilot is not already installed.
powershell -NoProfile -File tests/installer-smoke.ps1
```

The desktop executable is `target/release/personal-copilot-desktop.exe`. The unsigned NSIS installer is generated under `target/release/bundle/nsis/`. The binaries use Windows WebView2 and native inference. The native smoke script runs a separate app process, tests the actual WebView2 controls and Rust IPC, and writes screenshots to `target/ui-smoke/`.

The repository has a manually dispatched GitHub Actions workflow for portable core tests, Windows build and packaging, and dependency advisories. Physical audio and WebView2 screenshot checks run on a Windows desktop with known test inputs.

See the [3.0 validation report](validation/DESKTOP_3_0.md) for the Windows installer, real loopback, history, and visual results. The host's OS scale and the in-app text size are separate settings.

For a hardware smoke run, stop unrelated playback and set `COPILOT_TEST_MODEL` and `COPILOT_TEST_WAV` as described in the [audio comparator](benchmarks/audio.md). The native smoke test then also plays the synthetic fixture and checks the resulting transcript and question in the desktop workspace.

Get speech weights explicitly from the repository root. The script checks the pinned SHA-256 before accepting a model. The Rust engine independently verifies the exact bytes before loading them. Use the file chooser to select the result. Remove a model from the local `models/` folder to reclaim disk space. [Upstream model files](https://huggingface.co/ggerganov/whisper.cpp/tree/5359861c739e955e79d9a303bcbc70fb988958b1)

```powershell
powershell -NoProfile -File scripts/get-speech-model.ps1 -Model small
# Experimental candidate for the comparator.
powershell -NoProfile -File scripts/get-speech-model.ps1 -Model turbo
cargo run --release --locked -p copilot --features audio -- devices
```

## Remaining Windows checks

| Scenario | Pass condition |
| --- | --- |
| Clean install and uninstall | The unsigned installer opens the sample on a clean Windows 11 x64 user account with WebView2. Uninstall removes the app and its shortcuts. |
| Physical audio endpoints | Default and explicit routes work for speakers and a headset. Unplug and default switch produce clear recovery. A chosen microphone stays separate from output. |
| Long and difficult speech | Run consented or licensed Indian English and meeting-codec fixtures. Report quiet/short utterances, mixed speakers, technical vocabulary, gaps, and latency separately. |
| Desktop scaling and interaction | Inspect at OS 100% and 150%, with large and small windows. Check keyboard focus, long streams, scroll anchoring, and copied Markdown/code. |
| Connected providers | Verify approved account consent/denial, rate and usage limits, disconnect, data routing, and separate billing. |
