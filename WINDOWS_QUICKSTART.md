# Legacy Python setup on Windows

These instructions apply to the [archived Python revision](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/WINDOWS_QUICKSTART.md). For the current Windows app, see [Get started with OpenKeet](docs/getting-started.md) and [move your saved documents](docs/legacy-python.md).

## 1. Open the app

Open **OpenKeet (Legacy)** on your desktop. On a new checkout, open `start_windows.vbs` in the project folder once to create that shortcut.

The first launch creates `.venv` and installs the app dependencies. It uses an existing 64-bit Python 3.11 or 3.12 with Tk, or tries to install Python 3.12 through WinGet. It also tries to install native Claude Code if it is missing. Downloads need an internet connection. Later launches reuse the local environment.

## 2. Choose an answer model

Follow the [legacy model setup](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/docs/legacy-python.md#practice-with-a-question) and try a typed question with **Suggest answer**.

## 3. Listen to the other participant through system audio

1. Open **Audio**.
2. Wait for **small.en** to load. The shortcut starts this automatically. The first load downloads the model.
3. Select **System output (default)**. If your meeting app uses a different playback device, click **Refresh inputs** and select its specific **System output** entry.
4. Leave **Automatically answer speech** and **Draft answers while the question is still being asked** checked. **Question pause (seconds)** starts at 2.0 seconds. Raise it if the interviewer takes longer pauses.
5. Click **Start listening**, then return to **Assistant** to see the answer.

WASAPI loopback captures the selected speaker or headphone output. Match it to your meeting app's output and keep it unmuted. Play a short spoken clip and check the transcript. If the default output changes, restart listening. If an explicitly selected device disconnects, refresh inputs and select an output.

Rolling transcripts can trigger a provisional draft while the speaker continues. After the pause, the model checks the complete question and incorporates added constraints. If transcription lags, choose `base.en` or `tiny.en`.

Enter names or uncommon terms in **Speech hints** before starting a listening session. Use commas to separate entries. **Use full transcript as question** loads the entire box into the Assistant tab and pauses automatic answers while you review it. **Clear transcript** empties the Audio box while listening continues.

## 4. Screen sharing

On **Windows 10 version 2004 or newer / Windows 11**, the app applies and reads back capture exclusion before showing its main window, dropdowns, and file-path prompts. The persistent status reports success, failure or disabled state. While exclusion is enabled, paste a document path into the app's import prompt. For exports, enter a new path.

The assistant stays out of the taskbar and Alt+Tab. Press **Ctrl+Alt+I** while it is running to bring it forward. Select **Keep on top** if you want it to stay in view.

Use the right-edge scrollbar to scroll the current tab. When you scroll up inside an answer or transcript, new text preserves your reading position.

Test from the **receiving side** in the exact Meet, Zoom, or Teams sharing mode you plan to use. Use synthetic text and compare the receiver's view with exclusion off and on. Record the meeting application, share mode, Windows build, and visible result.

This is capture exclusion, not a universal undetectability guarantee. See [Microsoft's API documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity).

## If setup fails

- **Claude not found:** check the install message or use the [official Claude Code setup guide](https://code.claude.com/docs/en/setup).
- **Subscription not confirmed:** click **Sign in to Claude** and choose your subscription account.
- **No transcript:** verify the selected playback endpoint and try a short spoken clip. If the call plays through a different endpoint, select that output explicitly.
- **Package installation fails:** keep the terminal error text. Confirm that setup selected 64-bit Python 3.11 or 3.12 before retrying.

See [the validation record](VALIDATION.md) for observed Windows checks and the remaining hardware and receiving-side procedures.
