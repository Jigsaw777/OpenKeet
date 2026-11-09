# Get started with OpenKeet

Connect an answer model to practice typed questions without audio setup. To transcribe speech, download a speech model and choose what to listen to.

## 1. Build the Windows preview

Build the latest source to install the preview on **Windows 11 x64** with Microsoft Edge WebView2. Check Releases for versioned installers as they become available. [GitHub Releases](https://github.com/Jigsaw777/OpenKeet/releases)

1. Install Git, Node.js 24, and stable Rust for `x86_64-pc-windows-msvc`. In Visual Studio Installer, select **Desktop development with C++** with a Windows SDK and C++ CMake tools. Install LLVM with `libclang.dll` and add LLVM's `bin` folder to your `PATH`. [Toolchain setup and troubleshooting](desktop-slice.md#build-and-test)

2. Open **Developer PowerShell for Visual Studio** and clone `main` from the folder where you keep projects.

   ```powershell
   git clone --branch main --single-branch https://github.com/Jigsaw777/OpenKeet.git
   Set-Location OpenKeet
   ```

   If you already cloned OpenKeet, run `git switch main` and `git pull --ff-only` from its repository root.

3. From the OpenKeet repository root, build the installer. The first build downloads dependencies.

   ```powershell
   Set-Location apps/desktop
   if (-not $env:LIBCLANG_PATH) {
       $env:LIBCLANG_PATH = Split-Path -Parent (Get-Command clang.exe -ErrorAction Stop).Source
   }
   npm.cmd ci
   npm.cmd run desktop -- build --ci -- --locked
   ```

4. Run `OpenKeet_<version>_x64-setup.exe` from `target/release/bundle/nsis/` at the repository root. Finish setup, then search **OpenKeet** in Start. Source builds produce an unsigned installer. [Release status](release-readiness.md)

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
