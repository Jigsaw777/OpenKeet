# Move from the Python prototype

The current desktop app runs on Windows 11 x64 with a Rust backend and a React interface. The prototype and its launchers are available in the [last Python revision](https://github.com/Jigsaw777/OpenKeet/tree/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd). [Get started with OpenKeet](getting-started.md)

## Import your saved profile

1. Find the old profile at `%USERPROFILE%\.personal_copilot\profile.json` if you saved context in the prototype.
2. In **Prepare**, open **Your documents**. Choose the profile file and select **Add document**.
3. Review the imported resume, job description, and notes. Set your answer model in **Settings**, then **Answers**. Copy speech hints to **Words to recognize** under **Audio and display**.

The import reads a copy of the text into local prep storage. You can keep the original file as a backup. The current [provider setup](ai-providers.md) covers model choice and credentials.

## Run the text bridge

1. Select an answer model in OpenKeet. Quit OpenKeet from the tray.
2. In PowerShell, run `& "C:\path\to\OpenKeet.exe" --text-bridge` with your installed or built executable's path. Pass a port after the flag to override port 11435.
3. Set the desktop client's Ollama base URL to `http://127.0.0.1:11435` or the port you chose. Use the model name returned by `/api/tags`.

The bridge serves `/api/version`, `/api/tags`, and text requests at `/api/generate`. It uses the app's selected provider and forwards the submitted prompt. Cloud providers apply their usual data and billing terms. Start the bridge only when you want local desktop clients to use that connection. Quit OpenKeet to close the listener. [Provider guide](ai-providers.md)

## Source map

| Prototype file | Current implementation |
| --- | --- |
| `app.py` | [Tauri application](../apps/desktop/src-tauri/src/main.rs) and [React views](../apps/desktop/src/main.tsx) |
| `audio.py` | [WASAPI capture](../crates/copilot-audio/src/windows.rs) and [speech pipeline](../crates/copilot-audio/src/pipeline.rs) |
| `capture_privacy.py` | [Windows display affinity](../apps/desktop/src-tauri/src/platform.rs) |
| `core.py` | [Profile and document import](../apps/desktop/src-tauri/src/prep.rs) and [provider connections](../apps/desktop/src-tauri/src/providers.rs) |
| `free_cluely_bridge.py` | [Local text bridge](../apps/desktop/src-tauri/src/bridge.rs) |
| `live.py` | [Question and answer lifecycle](../crates/copilot-core/src/session.rs) |
| `providers.py` | [Ollama, Claude Console, and ChatGPT routes](../apps/desktop/src-tauri/src/providers.rs) |
| `window_shortcut.py` | [Tauri global shortcut and window restoration](../apps/desktop/src-tauri/src/main.rs) |
