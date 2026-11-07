# OpenKeet

## OpenKeet for Windows

OpenKeet runs on Windows 11 x64. It transcribes the other person's audio on your PC, spots their questions, and shows suggested answers in a see-through window on top of your call.

The start screen has three steps: what to **Listen to**, the local **Speech model**, and optional **Answers** from ChatGPT, Claude Console, or local Ollama. **Start listening** opens the see-through answer window; **Try a sample** plays an offline practice example. Install with `target/release/bundle/nsis/OpenKeet_0.1.1_x64-setup.exe`.

Use **Ctrl+Alt+I** or the OpenKeet tray icon to bring a hidden window back. Hiding pauses listening.

Stop a session before saving its retained transcript and checked answers to the local SQLite library. **History** supports transcript search, notes, Markdown/JSON/TXT export, and deletion.

See the [build and test guide](docs/desktop-slice.md), [provider authorization audit](docs/audit/PROVIDER_AUTHORIZATION.md), [3.0 validation report](docs/validation/DESKTOP_3_0.md), [paired performance comparison](docs/performance/PERFORMANCE_COMPARISON.md), and [feature parity matrix](docs/audit/FEATURE_PARITY_MATRIX.md).

## Legacy Python app

Personal desktop interview/meeting assistant with system-audio transcription, configurable Claude/Codex/local answers, editable resume and job context, and streamed output.

This download contains source code and launch scripts, not a prebuilt installer. Start with the standalone app for the complete audio-to-answer flow. An optional text bridge also connects the chat feature in the GitHub repository you supplied to Claude.

## What is included

| Capability | Implementation |
| --- | --- |
| Claude connection | Your installed, unmodified Claude Code CLI, signed in through its official subscription flow |
| Model choice | Sonnet by default, with Haiku, Opus, and supported custom Claude model IDs available |
| Codex connection | Your installed Codex CLI, signed in with ChatGPT; model configurable |
| Local alternative | Ollama with Qwen3 1.7B, 4B or 8B; no model-service subscription |
| System output | Windows WASAPI loopback; Linux PulseAudio/compatible PipeWire monitor; selected output |
| macOS audio | BlackHole or another virtual input route; explicit setup below |
| Transcription | Local faster-whisper with `small.en` by default and faster `base.en` and `tiny.en` options |
| Automatic responses | Rolling partial transcript → provisional draft; pause debounce → semantic question check → checked answer/revision |
| Personal context | Resume, job description and notes; TXT/MD/DOCX/text PDF imports |
| Window | Movable, resizable, optional always-on-top, with persistent Windows capture-exclusion status |
| Export | Completed questions and answers to Markdown |

No actual resume or employment history is preloaded. The Android/Kotlin, automotive audio, Java/backend and general presets change the coaching focus. They do not assert experience you have not supplied.

## 1. Connect Claude using your subscription

Install the latest Claude Code following [Anthropic's setup instructions](https://code.claude.com/docs/en/setup). Open a terminal and run:

```sh
claude update
claude auth login
claude auth status
```

Choose your personal Claude account rather than Console/API-key billing. Your plan must include Claude Code and the selected model. Make one ordinary test request in the CLI if needed to finish its initial onboarding.

The app invokes the official binary, lets it manage its own credentials, and never reads or copies your login tokens. Subscription mode removes inherited API-key/provider variables from the child process and rejects API-key/Console authentication. There is no automatic API fallback. Requests still consume your plan allowance. Account-level extra-usage settings remain controlled by the provider; this app cannot enforce an account spending cap. This is your personal end-user setup, not a Claude login service for other users.

Claude starts with the `sonnet` alias each time the app opens or you switch to Claude. You can choose another model for that session. Model access depends on your subscription. [Claude Code model aliases](https://code.claude.com/docs/en/model-config)

## 2. Install and run the app

Use Python **3.11 or 3.12** with Tk support. The audio environment pins NumPy below 2 for compatibility with the SoundCard backend. Keep these dependencies in the virtual environment.

### Windows

Open the **Personal Interview Assistant** desktop shortcut. On a new checkout, open `start_windows.vbs` once to create it. The launcher prepares the Python environment, opens the app, and loads the default speech model. See [Windows setup](WINDOWS_QUICKSTART.md).

### macOS / Linux

Run inside the extracted folder, using your Python 3.11/3.12 executable:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
.venv/bin/python app.py
```

After setup, `sh start_mac_linux.command` uses the virtual environment. On Linux, your OS may also need the Tk, PortAudio and PulseAudio client libraries, commonly named `python3-tk`, `libportaudio2` and `libpulse0`. On macOS use a Python installation with Tk support. You can check Tk with `python3 -m tkinter` before installing dependencies.

Typed questions and TXT/MD/DOCX imports can run with just Python/Tk; the requirements add PDF parsing and audio/transcription.

## 3. First run

1. Keep **Claude Code (subscription)** selected and click **Check connection**.
2. Open **Your context**. Paste/import your real resume, the target job description, and any preferences. Choose the relevant practice focus.
3. **Save context** is optional. It writes a local unencrypted profile. Unsaved edits also apply to answers.
4. Try a typed question first: “Explain structured concurrency and coroutine cancellation in Kotlin.” Press **Suggest answer** or Ctrl+Enter / Command+Enter.
5. Open **Audio**. The Windows shortcut loads `small.en` automatically. For a direct Python launch, click **Load speech model**.
6. Select the correct system-output route, then **Start listening**. **Automatically answer speech** and **Draft answers while the question is still being asked** are initially enabled. **Question pause (seconds)** starts at 2.0 seconds and can be set as high as 5.0 seconds.
7. Return to **Assistant** to see the answer stream in. Stop listening when finished.

You can switch provider and model in the main window. Available Claude model IDs may be typed directly. Only the listed local Qwen tags are accepted in the Ollama mode.

## 4. Capture the other participant's voice from system output

### Windows

Choose **System output (default)** or use **Refresh inputs** and select a specific **System output** entry. This records the selected playback endpoint using WASAPI loopback. It works with headphones when the call's audio is playing through that same endpoint. If your meeting app uses a different output from the Windows default, select its actual output explicitly.

Play a short spoken clip through the unmuted output and check that it appears in the transcript. SoundCard captures the native channels, and the app downmixes them to mono. If the Windows default changes while listening to the default, restart listening. If a selected output disconnects, refresh the list and choose an output before restarting.

### macOS

This version uses a virtual route rather than a packaged native CoreAudio Tap implementation:

1. Install [BlackHole 2ch](https://github.com/ExistentialAudio/BlackHole) using its official instructions.
2. Open **Audio MIDI Setup** and create a **Multi-Output Device** containing your headphones/speakers and BlackHole 2ch. Follow BlackHole's clock/drift-correction guidance for your devices.
3. Route the meeting's playback through that Multi-Output Device. You should still hear the other participant through your headphones.
4. In Personal Interview Assistant, refresh inputs and select **Input: BlackHole 2ch**.
5. Keep the meeting application's microphone set to your real microphone. Its playback route and microphone input serve different purposes.
6. Grant the requested microphone/audio permissions to the Python host or terminal. Test with a short audio clip.

Choosing “System output (default)” on macOS explains that a virtual input is required. It does not silently substitute microphone audio.

### Linux

Choose a system-output monitor through SoundCard. This requires PulseAudio or a compatible PipeWire PulseAudio service. If no monitors appear, confirm the sound server and the meeting's selected playback endpoint before retrying.

System-output capture can include your voice if a meeting app or sidetone plays it back. The app buffers short pauses and checks whether the speech is a continuation, a new question, or incomplete. In **Audio**, click **Use full transcript as question** to load the whole transcript and pause automatic answers while you review it. **Clear transcript** empties the Audio box while listening continues.

## 5. Screen-share visibility

**No universal “undetectable” guarantee is made.** Transparency, always-on-top and hiding a taskbar icon do not establish screen-share privacy.

| Platform | What this app provides | Practical check |
| --- | --- | --- |
| Windows 10 version 2004+ / Windows 11 | Optional `WDA_EXCLUDEFROMCAPTURE` with readback on the main window, dropdowns, and file-path dialogs | Test the receiving view in the exact meeting application and sharing mode |
| macOS | Standard visible desktop window | Share one specific app window or use a separate unshared display |
| Linux | Standard desktop window; compositor-dependent capture behavior | Use a specific shared window or an unshared display and verify the receiving view |

On Windows the app checks the native handle before showing a window, then checks it again on map and configuration events. A persistent status reports success, failure or disabled state. With exclusion enabled, use the app's protected path prompt to import or export files. Exports through that prompt require a new file path. Verify the receiving view in your meeting app and share mode.

The Windows assistant uses a tool window so it stays out of Alt+Tab and the taskbar. Press **Ctrl+Alt+I** while the app is running to bring it forward. Use **Keep on top** to keep it in view. The right-edge scrollbar moves the current tab, and streaming answers and transcripts follow new text only while you are at the end.

Microsoft describes display affinity as protection through a particular set of capture APIs, not a comprehensive guarantee. Electron likewise documents that newer macOS apps using ScreenCaptureKit can capture a protected Electron window. The GitHub reference's `setContentProtection(true)` should therefore not be treated as a universal invisibility feature.

## 6. Response speed

The implemented path is:

**selected playback/input → local speech detection → local Whisper transcription → selected model → streamed text**.

While speech continues, cumulative audio snapshots are offered to local Whisper about every 0.9 seconds. Old queued snapshots are replaced by the newest one if recognition falls behind. Once the transcript contains enough wording, a provisional model draft can start before the speaker finishes. About 750 ms of silence closes a capture segment. The question buffer can still join it with the next segment.

Checked answers wait for the **Question pause** debounce, initially 2.0 seconds since the last captured speech. Unfinished clauses wait at least 2.2 seconds. Resumed speech blocks the pending final request even before its next transcript arrives. The app merges continuations, then the selected answer model classifies the candidate as **new**, **continue** or **incomplete**. Checked answer text appears after the app validates the decision header.

Early predictions remain labeled provisional. Added wording marks an earlier answer outdated. When the revised question is ready, a stale in-flight answer is stopped and replaced. Drafts, obsolete answers and incomplete questions are excluded from saved conversation history. A continuation replaces the earlier question/answer pair. The app normally makes one early draft attempt; it permits one more after substantial new wording if the first correctly responded that the actual question was not yet clear. See `LIVE_QUESTION_LOGIC.md`.

Use `small.en` for better transcription accuracy. The local recognizer uses technical vocabulary hints and checks for speech on completed segments. Add names or uncommon terms in **Audio → Speech hints** before starting listening. Save them with **Your context → Save context on this computer**. If rolling transcripts lag, select `base.en` or `tiny.en`.

Keep the speech model loaded, use concise mode and a short relevant résumé or job excerpt. The local Qwen model disables thinking output and stays warm for ten minutes after a request. Claude requests disable extended thinking and run without ordinary tools or customizations. Subscription-auth checks are cached for five minutes.

Each CLI answer starts a new process, so CLI startup, server/network delay, transcription and model generation all add time. Codex JSON output may deliver a complete agent message rather than token-by-token text. This makes Claude the better initial interface to test for progressive display; it does not establish a benchmark win for any model.

The status bar reports **first-text time from the model request** and, for an automatically submitted speech question, **time since the last captured speech block**. Checked-answer timing includes generating the semantic decision header. Full-answer time is shown separately. These measurements include the app's event-loop scheduling. A short question can finish before its draft arrives; a guaranteed answer before the speaker finishes is not possible. No “instant” response claim has been validated on your hardware.

## 7. Use your Codex subscription instead

Install [Codex CLI](https://learn.chatgpt.com/docs/cli/reference), then run:

```sh
codex login
codex login status
```

Use **Sign in with ChatGPT**. Select **Codex CLI (subscription)** in the app. `default` uses the CLI default model; you can enter another model ID available to your account. This adapter runs in an ephemeral temporary directory with a read-only sandbox, disables web search and requests low reasoning effort. Your normal coding repository is not its working directory.

Both CLI modes require an updated CLI. An unavailable model, expired login, rate limit or incompatible CLI produces an error; the app does not silently switch to paid API access.

## 8. Fully local alternative

Install [Ollama](https://ollama.com), download a supported local model, and start Ollama:

```sh
ollama pull qwen3:4b
```

Select **Ollama (local)** and `qwen3:4b`. The 1.7B option is lighter; the 8B option needs more memory. Download size alone is not the runtime RAM requirement. Benchmark with your other applications open.

The app's Ollama client only connects to `127.0.0.1:11434`, rejects redirects and cloud-marked model metadata, and uses the listed model tags. For stronger local-only enforcement, follow [Ollama's cloud-disable instructions](https://docs.ollama.com/faq): configure `disable_ollama_cloud` in its server configuration or set `OLLAMA_NO_CLOUD=1` for the **Ollama service**, then restart it. Setting that variable only for this GUI does not change an already running Ollama service.

With downloaded local Whisper and Qwen models, inference can run offline. Cloud CLI modes send your question, context and bounded conversation history to the selected provider.

## 9. Connect the linked free-cluely repository to Claude

The repository was reviewed at commit `712ddb3474af22886cd47786f7bb0f0a5bf526df`. Its text-chat Ollama path posts to `/api/generate`. The included bridge exposes that text interface while using your official Claude/Codex CLI login behind it.

In the Personal Interview Assistant folder, run:

```sh
python free_cluely_bridge.py --provider claude --model sonnet
```

Use your virtual environment's Python executable if needed. Leave that terminal running. In the root of your separately downloaded [Prat011/free-cluely](https://github.com/Prat011/free-cluely) checkout, set these `.env` entries:

```dotenv
USE_OLLAMA=true
OLLAMA_URL=http://127.0.0.1:11435
OLLAMA_MODEL=sonnet
```

Then start that repository according to its README. The reviewed source imports `dotenv` although its root package manifest does not list it directly; if the build reports it missing, install it in that checkout with `npm install dotenv`.

**Scope of this integration:** it connects the upstream **text-chat path** to Claude. The UI will still call the transport “Ollama.” It does not turn that repository's image/audio Gemini calls into Claude calls, implement native loopback capture there, or make its existing non-streaming UI display tokens progressively. For continuous system audio → Claude → streamed answers, use the standalone `app.py` in this download. If you save a resume/JD in the standalone app, the text bridge can use that same local profile.

To use Codex with the bridge, pass `--provider codex --model default` and set `OLLAMA_MODEL=default`. The bridge processes one request at a time on loopback and rejects browser Origin headers.

The bridge's startup/API compatibility was tested with fake model providers. The upstream Electron app was not installed or run in this environment.

## 10. Local data and limits

- Context is saved only when you click Save, to `~/.personal_copilot/profile.json`. It is unencrypted. POSIX permissions are restricted on creation; Windows uses the user-directory ACLs.
- Audio samples stay in bounded in-memory queues. This app does not write WAV/MP3 recordings. Speech-model weights are cached by faster-whisper/Hugging Face.
- Questions, transcripts and answers stay in session memory until cleared/closed, unless you explicitly export. Official CLI/provider diagnostics, account metadata and retention are governed by their settings and policies; disabling session persistence is not a promise about every provider log.
- Cloud providers receive the selected text context. API keys and login tokens are not stored in this app's profile.
- Résumé and job description fields accept up to 12,000 characters each. Notes accept 3,000 and typed questions accept 4,000. The full transcript button copies all visible text. Submitting it accepts up to 64,000 characters for Claude and Codex, or 12,000 for Ollama.
- No screenshots, screen OCR, native macOS process tap, speaker separation, cloud speech recognition, packaged installer, or bypass of meeting/proctoring software is included.
- Use recording and AI assistance only in sessions where you have permission to use them.

## Verification and troubleshooting

```sh
python app.py --diagnostics
```

See [Windows setup](WINDOWS_QUICKSTART.md) to run the app and [validation](VALIDATION.md) for manual checks.

If no transcript appears, confirm the selected endpoint, output routing, OS permission, packages and speech threshold. If transcription lags, try tiny.en and shorter utterances. If Claude is missing, start the app from a terminal where `claude auth status` succeeds so it inherits the same PATH.

## Sources

- Reviewed free-cluely's Electron window protection, five-second microphone recording, and model dispatch, and Natively's per-window content protection and model handling. This Python project uses its existing audio and provider pipeline. The review supplied behavioral reference only. [free-cluely (Apache-2.0)](https://github.com/Prat011/free-cluely), [Natively (Personal Use Source License)](https://github.com/Natively-AI-assistant/natively-cluely-ai-assistant)
- [Claude Code programmatic mode](https://code.claude.com/docs/en/headless), [CLI reference](https://code.claude.com/docs/en/cli-reference), [model configuration](https://code.claude.com/docs/en/model-config), and [subscription sign-in](https://support.claude.com/en/articles/11145838-use-claude-code-with-your-pro-max-or-ultra-plan).
- [Codex authentication](https://learn.chatgpt.com/docs/auth), [non-interactive mode](https://learn.chatgpt.com/docs/non-interactive-mode), and [CLI reference](https://learn.chatgpt.com/docs/cli/reference).
- [faster-whisper](https://github.com/SYSTRAN/faster-whisper), [SoundCard](https://soundcard.readthedocs.io/en/latest/), and [BlackHole](https://github.com/ExistentialAudio/BlackHole).
- [Microsoft SetWindowDisplayAffinity](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity), [Electron capture limitations](https://www.electronjs.org/docs/latest/api/browser-window), and [Ollama chat API](https://docs.ollama.com/api/chat).
