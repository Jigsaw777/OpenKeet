# Troubleshoot OpenKeet

Try **Try a sample** first. If the sample works, the app is running and you can narrow the problem to audio or the answer model.

| What you see | What to try |
| --- | --- |
| Build or install fails | Use Windows 11 x64. Check WebView2, the MSVC C++ tools and Windows SDK, CMake, LLVM/libclang, Rust, and Node.js 24. The [build guide](desktop-slice.md#build-and-test) explains `LIBCLANG_PATH`. Use the exact first error in a bug report. |
| The window disappears or startup seems stuck | Check the Windows tray for **Show OpenKeet**. Try **Ctrl+Alt+I**. If the app never appears, check the build or installer output and the WebView2 runtime. |
| Microphone is missing | Plug it in, check that Windows lists it as an input, allow desktop-app microphone access, then refresh the device list. |
| Playback is silent in OpenKeet | Play a short speech clip. Under **Listen to**, select the same speakers or headset your call uses. If the device was unplugged, reconnect it or stop and choose a new device. |
| Listening starts but text is missing | Check the input meter and the selected source. Wait for **Speech model** to finish downloading. If you picked a model manually, use a [supported file](desktop-slice.md#build-and-test). |
| No answer appears | Try **Get answer** or **Ask a question**. Under **Settings**, open **Answers** and check which provider is active. |
| Ollama model list is empty | Start Ollama, download a local model, and reload the list. Cloud models are filtered out. |
| Claude key is missing or rejected | Save a current Console API key from an enabled workspace. Check workspace billing and model access. Keep the key out of bug reports. |
| ChatGPT sign-in fails or usage is limited | Finish browser consent, then check that the account is eligible and has usage available. If needed, try another answer route. |
| High memory use or slow answers | Try a smaller Ollama model or the standard Whisper model. Close other heavy apps and retry a short question. The offline sample is a quick fallback when a model won't run on your PC. |

If the problem persists, see [Support](../SUPPORT.md) for what to include and where to report it.
