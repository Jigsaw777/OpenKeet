# Get started with OpenKeet

Try the bundled rehearsal first. It uses prepared questions and answers and needs no microphone, account, or model download.

## 1. Build the Windows preview

The desktop preview targets **Windows 11 x64** with Microsoft Edge WebView2. There is no published installer. Use a checkout you have access to and install Node.js 24, stable Rust with the Windows MSVC target, Visual Studio C++ build tools and Windows SDK, CMake, and LLVM/libclang. The build needs internet access for dependencies. [Toolchain setup and build troubleshooting](desktop-slice.md#build-and-test)

From the repository root in PowerShell:

```powershell
Set-Location apps/desktop
npm ci
npm run desktop -- build --ci -- --locked
```

Open the generated `OpenKeet_*_x64-setup.exe` under `target/release/bundle/nsis/` at the repository root. The build produces an unsigned Windows preview. You can also run `target/release/OpenKeet.exe` from the build output. [Release status](release-readiness.md)

## 2. Try a sample

1. Launch **OpenKeet** and select **Try a sample**.
2. Read the example transcript and checked answers. Expand an answer to focus on its explanation.
3. When the sample ends, choose **Save to history** if you want to keep it.

## 3. Practice your own questions

1. Open **Settings**, then **Answers**. Choose an [answer model](ai-providers.md) and select the **Use** button for that provider.
2. Return to **Session**, open **Ask a question**, and type a question you expect to hear.
3. Read the response, then try explaining the idea aloud. [Practice examples](interview-preparation.md)

## 4. Practice with audio

1. Under **Listen to**, select your microphone to practice alone or the speakers or headphones playing a consented mock interview. In Windows privacy settings, allow desktop apps to use the microphone if you choose it.
2. Under **Speech model**, select **Download** for the recommended local model or **Choose file** for a supported download. The standard English model takes about 465 MiB on disk.
3. Select **Start listening** and confirm you have permission. For suggestions, choose **Get answer** when OpenKeet detects a question. You can choose **Transcription only** to use the app without an answer model.
4. Select **Stop** when finished. Saving to **History** is optional.

If the app can't hear you or you get a setup error, start with [troubleshooting](troubleshooting.md).
