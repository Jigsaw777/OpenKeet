# Windows setup

Use the standalone app in this folder for system-audio transcription and automatic Claude answers. The optional GitHub text bridge is not needed for this flow.

## 1. Prepare Python and Claude Code

This build uses **64-bit Python 3.11 or 3.12**, with Tk and the Windows Python launcher. If you already have one of those versions installed, keep it. The setup script checks the supported versions before creating its environment. It does not modify your system Python or install packages globally.

Install **Claude Code natively on Windows** so this Windows app can launch it directly. A CLI installed only inside WSL is a separate Linux executable and is not the native Windows CLI used here.

In PowerShell, the official WinGet install command is:

```powershell
winget install Anthropic.ClaudeCode
```

If Claude Code is already installed with WinGet, use `winget upgrade Anthropic.ClaudeCode` to update. Then **close and reopen PowerShell** so the new executable is on PATH.

```powershell
claude --version
claude auth login
claude auth status
```

Sign in with the same personal Claude subscription account you use for Claude. Do not select Console/API billing for this app's subscription mode. If Claude has never been used on this computer, launch `claude` once and complete its ordinary first-run setup before returning here. Your selected model must be included in your plan. Account usage limits and extra-usage settings still apply.

Anthropic also offers its native PowerShell installer and documents Windows PATH troubleshooting at [Claude Code setup](https://code.claude.com/docs/en/setup). The login commands are documented in the [CLI reference](https://code.claude.com/docs/en/cli-reference).

## 2. Install this app's dependencies

1. Extract the complete ZIP into a normal writable folder.
2. Open the extracted `personal_copilot` folder.
3. Double-click **setup_windows.bat**. The first run downloads its Python dependencies into `.venv` beside the app. Keep the complete extracted folder together.
4. Once setup succeeds, double-click **start_windows.vbs** for a console-free launch.

If you prefer PowerShell commands, run these from the extracted folder:

```powershell
py -3.12 -m venv .venv
.\.venv\Scripts\python.exe -m pip install -r requirements.txt
.\.venv\Scripts\python.exe app.py
```

Use `-3.11` if that is your installed supported version. No PowerShell activation script or execution-policy change is required.

## 3. Configure Claude and personal context

1. Select **Claude Code (subscription)** and model **haiku**.
2. Click **Check connection**.
3. Open **Your context** and enter/import your real resume, target job description and preferences.
4. Try a typed question before enabling audio. For more involved technical questions, compare `sonnet` with `haiku` using the model control.

## 4. Listen to the other participant through system audio

1. Open **Audio**.
2. Select **tiny.en**, then click **Load speech model**. It downloads once and transcribes locally afterwards.
3. Select **System output (default)**. If your meeting app uses a different playback device, click **Refresh inputs** and select its specific **System output** entry.
4. Leave **Automatically answer speech** and **Draft answers while the question is still being asked** checked. Set **Question pause (seconds)** to tune the debounce; it starts at 1.2 seconds.
5. Click **Start listening**, then return to **Assistant** to see the answer.

WASAPI loopback captures the selected speaker or headphone output. Match it to your meeting app's output and keep it unmuted. Play a short spoken clip and check the transcript. If the default output changes, restart listening. If an explicitly selected device disconnects, refresh inputs and select an output.

Rolling transcripts can trigger a provisional draft while the speaker continues. A short pause does not automatically end the question: the buffer joins continuations, then the model checks the consolidated question after the debounce. Added constraints replace obsolete predictions. The app shows measured first-text and full-answer times. CLI startup, transcription and network/model time contribute, so an instant answer is not promised. Switch to `base.en` if tiny.en misses technical vocabulary or names.

## 5. Screen sharing

On **Windows 10 version 2004 or newer / Windows 11**, the app applies and reads back capture exclusion before showing its main window, dropdowns, and file-path prompts. The persistent status reports success, failure or disabled state. While exclusion is enabled, paste a document path into the app's import prompt. For exports, enter a new path.

Test from the **receiving side** in the exact Meet, Zoom, or Teams sharing mode you plan to use. Use synthetic text and compare the receiver's view with exclusion off and on. Record the meeting application, share mode, Windows build, and visible result.

This is capture exclusion, not a universal undetectability guarantee. See [Microsoft's API documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity).

## If setup fails

- **Claude not found:** confirm `claude --version` works in a newly opened PowerShell window, then launch the app from that window.
- **Subscription not confirmed:** check `claude auth status`; update the CLI and use subscription sign-in rather than API-key/Console credentials.
- **No transcript:** verify the selected playback endpoint and try a short spoken clip. If the call plays through a different endpoint, select that output explicitly.
- **Package installation fails:** keep the terminal error text. Confirm that setup selected 64-bit Python 3.11 or 3.12 before retrying.

See [the validation record](VALIDATION.md) for observed Windows checks and the remaining hardware and receiving-side procedures.
