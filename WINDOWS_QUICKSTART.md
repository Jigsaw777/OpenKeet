# Windows setup

Use the standalone app in this folder for system-audio transcription and automatic Claude answers. The optional GitHub text bridge is not needed for this flow.

## 1. Open the app

Open **Personal Interview Assistant** on your desktop. On a new checkout, open `start_windows.vbs` in the project folder once to create that shortcut.

The first launch creates `.venv` and installs the app dependencies. It uses an existing 64-bit Python 3.11 or 3.12 with Tk, or tries to install Python 3.12 through WinGet. It also tries to install native Claude Code if it is missing. Downloads need an internet connection. Later launches reuse the local environment.

## 2. Connect Claude

If the app asks you to sign in, click **Sign in to Claude**. Complete the official subscription sign-in in your browser. Keep the sign-in window open in case Claude asks you to paste a code there. The app checks the connection when sign-in finishes.

Type a question and click **Suggest answer**. You can add résumé, job description, and preference text in **Your context**.

## 3. Listen to the other participant through system audio

1. Open **Audio**.
2. Wait for **small.en** to load. The shortcut starts this automatically. The first load downloads the model.
3. Select **System output (default)**. If your meeting app uses a different playback device, click **Refresh inputs** and select its specific **System output** entry.
4. Leave **Automatically answer speech** and **Draft answers while the question is still being asked** checked. Set **Question pause (seconds)** to tune the debounce; it starts at 1.2 seconds.
5. Click **Start listening**, then return to **Assistant** to see the answer.

WASAPI loopback captures the selected speaker or headphone output. Match it to your meeting app's output and keep it unmuted. Play a short spoken clip and check the transcript. If the default output changes, restart listening. If an explicitly selected device disconnects, refresh inputs and select an output.

Rolling transcripts can trigger a provisional draft while the speaker continues. After the pause, the model checks the complete question and incorporates added constraints. If transcription lags, choose `base.en` or `tiny.en`.

## 4. Screen sharing

On **Windows 10 version 2004 or newer / Windows 11**, the app applies and reads back capture exclusion before showing its main window, dropdowns, and file-path prompts. The persistent status reports success, failure or disabled state. While exclusion is enabled, paste a document path into the app's import prompt. For exports, enter a new path.

The assistant stays out of the taskbar and Alt+Tab. Select **Keep on top** to keep the window in view while you switch apps.

Test from the **receiving side** in the exact Meet, Zoom, or Teams sharing mode you plan to use. Use synthetic text and compare the receiver's view with exclusion off and on. Record the meeting application, share mode, Windows build, and visible result.

This is capture exclusion, not a universal undetectability guarantee. See [Microsoft's API documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity).

## If setup fails

- **Claude not found:** check the install message or use the [official Claude Code setup guide](https://code.claude.com/docs/en/setup).
- **Subscription not confirmed:** click **Sign in to Claude** and choose your subscription account.
- **No transcript:** verify the selected playback endpoint and try a short spoken clip. If the call plays through a different endpoint, select that output explicitly.
- **Package installation fails:** keep the terminal error text. Confirm that setup selected 64-bit Python 3.11 or 3.12 before retrying.

See [the validation record](VALIDATION.md) for observed Windows checks and the remaining hardware and receiving-side procedures.
