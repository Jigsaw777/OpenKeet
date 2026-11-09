# Validation

These results apply to the Python prototype. See the [migration guide](docs/legacy-python.md) for the current Windows app and its test paths.

November 4, 2026. Windows 11 25H2, build 26200.9711, Python 3.12.10.

## Windows checks

| Check | Method | Observed result |
| --- | --- | --- |
| Display affinity | Native Windows readback before show and after map | Main window, file-path dialog, and dropdown returned `0x11`. Reposting after recreating the dropdown's HWND also returned `0x11` before it mapped. |
| Desktop shortcut | Windows Script Host and PowerShell launcher | Built a fresh virtual environment, installed dependencies, and opened the app under `pythonw.exe`. The second launch reused the environment. The main window returned `0x11`, and `tiny.en` loaded before listening started. Python and Claude Code were already installed, so the missing-tool WinGet paths were not exercised. |
| Local recording | FFmpeg 9.0.2, `gdigrab` desktop-region recording, 10 fps, 8 frames per run | The assistant covered the test background with exclusion off. With exclusion on, all recorded frames showed the background while the assistant remained mapped and its text remained visible locally. |
| Controlled screen share | Chrome 156.0.8078.80, full-display `getDisplayMedia`, separate local WebRTC receiving tab | The decoded video showed the synthetic answer with exclusion off. After fresh frames arrived with exclusion on, the receiver showed only the background in that region. The app remained mapped on the sending display. |
| WASAPI | Windows default and explicit `Speaker (Realtek(R) Audio)` output. Synthetic playback and a fixture recognizer. | Silence produced no transcript events. Playback produced a rolling partial and final. Stop, restart, and second stop completed on both routes. The test restored the endpoint's initial mute state. SoundCard reported data discontinuity warnings during playback transitions. |
| Local transcription | Default Realtek output and real `tiny.en` Whisper | A spoken process-versus-thread question and a follow-up constraint each produced rolling partials and a correct final transcript. |
| Other receiving views | Second device, Meet, Teams, or Zoom | Unverified. The Chrome test used two local tabs. |
| Headset and physical endpoint changes | Headset and a second playback endpoint | Unverified. Windows exposed one playback endpoint. Signed-in provider behavior also remains unverified. |

## November 5 update

- The visible assistant window and file-path prompt had the native Windows tool-window flag, with the taskbar-forcing flag cleared and capture affinity at `0x11`. Direct inspection of the taskbar and Alt+Tab views remains pending.
- On five synthetic technical questions totaling 28.29 seconds, `small.en` made 1 word error out of 68. `base.en` and `tiny.en` each made 7. Decode times were 12.32, 3.50, and 1.84 seconds, respectively, on this PC.
- The default `small.en` produced a rolling partial before the end of a 6.48-second WASAPI playback clip and correctly transcribed “idempotency keys” in its final. The faster models misspelled that term on the same samples. A separate loopback clip still produced “codelink” for “Kotlin.” The probe restored the endpoint's initial mute state.

## Window recall and pause checks

- Pressing **Ctrl+Alt+I** from a separate foreground app restored and focused the hidden assistant. The visible window still had the tool-window style and capture affinity `0x11`.
- In a compact window, the Assistant and Audio tabs scrolled. The answer and transcript held their reading positions during streaming and followed new text when already at the end.
- With `small.en`, a synthetic question containing 0.60- and 1.60-second pauses yielded one checked question with the follow-up constraint intact. Controlled timestamps also verified a 1.75-second pause, speech resuming before its transcript, and a distinct next question.

## Transcript accuracy and controls

- On synthetic audio, the previous decoder wrote “LRU Cash” for a short fragment. With speech hints and the updated final decoder, `small.en` produced “LRU cache,” kept “cash” in a business question, and returned no completed transcript for a non-speech tone. A supplied local name hint preserved its spelling. The original interview audio was unavailable for comparison.
- A Windows app check copied the full transcript despite an active selection and sent more than 4,000 characters through a test provider. Clearing while listening kept delayed results for earlier speech out of the cleared box. Sonnet was selected on startup and when switching back to Claude.

## Complete the remaining Windows checks

1. Open the **OpenKeet (Legacy)** desktop shortcut and put synthetic text in the assistant. In a test meeting, share the full display to a second device. Compare the receiver's view with exclusion off and on. Repeat with window sharing if you use that mode.
2. Open an assistant dropdown and an import path prompt during the test. Record the Windows build, meeting app and version, share mode, and what each device displays.
3. Connect a headset, route playback to it, refresh inputs, and select its system output. Load a speech model and play a known spoken clip. Confirm a rolling partial, a completed transcript, and no transcript during silence.
4. Change the Windows default while listening to **System output (default)**. Confirm the app asks you to restart and then listens to the new default. Repeat with the headset explicitly selected and unplug it. Confirm a visible error, then refresh, select an available output, and restart.
5. Check a signed-in provider and play a question with a late constraint. Confirm the provisional draft gives way to a checked revision and the session export contains the checked answer.
