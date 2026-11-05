# Start here

Use **app.py** for the full system-audio → transcription → Claude answer flow.

**Windows:** open the **Personal Interview Assistant** desktop shortcut. On a new checkout, open `start_windows.vbs` once to create it. The launcher handles Python setup and preloads speech recognition. If Claude asks for a login, click **Sign in to Claude** in the app. See [Windows setup](WINDOWS_QUICKSTART.md).

**macOS and Linux:** follow the Python environment steps in [README](README.md), then launch `app.py`.

1. Select your provider and try a typed question with **Suggest answer**. Add optional résumé or job context under **Your context**.
2. Open **Audio**, choose the output carrying the other person's voice, and click **Start listening**. On a direct Python launch, load `small.en` first. macOS needs the BlackHole route explained in the README.

**Model:** Claude starts with `sonnet`. The provider and model controls also support Haiku, Opus, Codex, and local Ollama.

**Screen sharing:** Windows checks capture exclusion before showing the main window, dropdowns, and file-path prompts. Verify the receiving view in the exact meeting app and share mode.

**Your GitHub link:** To connect that repository's text chat to Claude, run `free_cluely_bridge.py` and use the three `.env` values in README section 9. That bridge handles text chat; it does not upgrade the upstream audio/image pipeline. Use the standalone app for system-audio assistance.

**Status:** See [validation](VALIDATION.md) for the automated results, observed Windows tests, and remaining hardware and sharing checks.

**Cost:** This code has no app subscription. Claude/Codex requests use the connected service's allowance and limits. Local Ollama is the option without cloud-model service usage. The app never falls back automatically to a separately billed API key.
