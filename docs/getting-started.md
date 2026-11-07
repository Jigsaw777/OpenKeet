# Get started with OpenKeet

Connect an answer model to practice typed questions without audio setup. To transcribe speech, download a speech model and choose what to listen to.

## 1. Build the Windows preview

The desktop preview targets **Windows 11 x64** with Microsoft Edge WebView2. There is no published installer. Use a checkout you have access to and install Node.js 24, stable Rust with the Windows MSVC target, Visual Studio C++ build tools and Windows SDK, CMake, and LLVM/libclang. The build needs internet access for dependencies. [Toolchain setup and build troubleshooting](desktop-slice.md#build-and-test)

From the repository root in PowerShell:

```powershell
Set-Location apps/desktop
npm ci
npm run desktop -- build --ci -- --locked
```

Open the generated `OpenKeet_*_x64-setup.exe` under `target/release/bundle/nsis/` at the repository root. The build produces an unsigned Windows preview. You can also run `target/release/OpenKeet.exe` from the build output. [Release status](release-readiness.md)

## 2. Practice your own questions

1. Open **Settings**, then **Answers**. Choose an [answer model](ai-providers.md) and select the **Use** button for that provider.
2. Return to **Session**, open **Ask a question**, and type a question you expect to hear.
3. Read the response, then try explaining the idea aloud. [Practice examples](interview-preparation.md)

## 3. Practice with audio

1. Under **Listen to**, select your microphone to practice alone or the speakers or headphones playing a call where everyone has agreed to transcription. In Windows privacy settings, allow desktop apps to use the microphone if you choose it.
2. Under **Speech model**, select **Download (488 MB)** for the default English Whisper model or **Choose file** if you already have the supported `ggml-small.en.bin`.
3. Once you have permission, select **Start listening**. If an answer model is connected, review its suggestions or use **Get answer** for a detected question. Select **Transcription only** in **Settings**, under **Answers**, if you only need a transcript.
4. Select **Stop** when finished. Saving to **History** is optional.

To try experimental **NVIDIA Parakeet v3**, open **Settings**, then **Audio and display**, and select it under **Speech engine**. Return to **Session** and choose **Download Parakeet (670 MB)** or **Choose folder** for the supported model folder. Both speech engines run on your PC after setup. **Words to recognize** and **Short follow-up mode** apply to Whisper only.

If the app can't hear you or you get a setup error, start with [troubleshooting](troubleshooting.md).
