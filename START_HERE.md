# Start here

Use **app.py** for the full system-audio → transcription → Claude answer flow.

**Windows:** read `WINDOWS_QUICKSTART.md`. After installing the supported Python and signing in to the native Claude Code CLI, run `setup_windows.bat` once and double-click `start_windows.vbs` to launch.

1. Install/update **Claude Code** from [its official setup page](https://code.claude.com/docs/en/setup). Run `claude auth login` and sign in with your Claude subscription, then check `claude auth status`.
2. Install **Python 3.11 or 3.12 with Tk support**. Create a virtual environment in this extracted folder and install `requirements.txt`. The exact Windows/macOS/Linux commands are in `README.md`.
3. Run `app.py`. Leave **Claude Code (subscription)** selected, click **Check connection**, add your real resume and job description under **Your context**, and try a typed question.
4. Open **Audio**, preload `tiny.en`, choose the output carrying the other person's voice, and click **Start listening**. Automatic answers and live drafts are initially enabled. **Question pause** starts at 1.2 seconds; continuation handling and a model question check keep a brief pause from automatically becoming a new question. Windows/Linux use system loopback; macOS needs the BlackHole route explained in the README.

**Model:** Start with `haiku` to test speed; switch to `sonnet` to compare technical-answer quality. The provider/model controls also support Codex and local Ollama. The app shows measured first-text and full-answer times; zero delay is not guaranteed.

**Screen sharing:** Windows checks capture exclusion before showing the main window, dropdowns, and file-path prompts. Verify the receiving view in the exact meeting app and share mode.

**Your GitHub link:** To connect that repository's text chat to Claude, run `free_cluely_bridge.py` and use the three `.env` values in README section 9. That bridge handles text chat; it does not upgrade the upstream audio/image pipeline. Use the standalone app for system-audio assistance.

**Status:** See [validation](VALIDATION.md) for the automated results, observed Windows tests, and remaining hardware and sharing checks.

**Cost:** This code has no app subscription. Claude/Codex requests use the connected service's allowance and limits. Local Ollama is the option without cloud-model service usage. The app never falls back automatically to a separately billed API key.
