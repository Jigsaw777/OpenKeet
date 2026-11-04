# Validation

November 4, 2026. Windows 11 25H2, build 26200.9711, Python 3.12.10.

## Windows checks

| Check | Method | Observed result |
| --- | --- | --- |
| Display affinity | Native Windows readback before show and after map | Main window, file-path dialog, and dropdown returned `0x11`. Reposting after recreating the dropdown's HWND also returned `0x11` before it mapped. |
| Launcher | Windows Script Host, `start_windows.vbs` | Started the app under `pythonw.exe`. The visible main window returned `0x11`. |
| Local recording | FFmpeg 9.0.2, `gdigrab` desktop-region recording, 10 fps, 8 frames per run | The assistant covered the test background with exclusion off. With exclusion on, all recorded frames showed the background while the assistant remained mapped and its text remained visible locally. |
| Controlled screen share | Chrome 156.0.8078.80, full-display `getDisplayMedia`, separate local WebRTC receiving tab | The decoded video showed the synthetic answer with exclusion off. After fresh frames arrived with exclusion on, the receiver showed only the background in that region. The app remained mapped on the sending display. |
| WASAPI | Windows default and explicit `Speaker (Realtek(R) Audio)` output. Synthetic playback and a fixture recognizer. | Silence produced no transcript events. Playback produced a rolling partial and final. Stop, restart, and second stop completed on both routes. The test restored the endpoint's initial mute state. SoundCard reported data discontinuity warnings during playback transitions. |
| Local transcription | Default Realtek output and real `tiny.en` Whisper | A spoken process-versus-thread question and a follow-up constraint each produced rolling partials and a correct final transcript. |
| Other receiving views | Second device, Meet, Teams, or Zoom | Unverified. The Chrome test used two local tabs. |
| Headset and physical endpoint changes | Headset and a second playback endpoint | Unverified. Windows exposed one playback endpoint. Signed-in provider behavior also remains unverified. |

## Complete the remaining Windows checks

1. Launch `start_windows.vbs` and put synthetic text in the assistant. In a test meeting, share the full display to a second device. Compare the receiver's view with exclusion off and on. Repeat with window sharing if you use that mode.
2. Open an assistant dropdown and an import path prompt during the test. Record the Windows build, meeting app and version, share mode, and what each device displays.
3. Connect a headset, route playback to it, refresh inputs, and select its system output. Load a speech model and play a known spoken clip. Confirm a rolling partial, a completed transcript, and no transcript during silence.
4. Change the Windows default while listening to **System output (default)**. Confirm the app asks you to restart and then listens to the new default. Repeat with the headset explicitly selected and unplug it. Confirm a visible error, then refresh, select an available output, and restart.
5. Check a signed-in provider and play a question with a late constraint. Confirm the provisional draft gives way to a checked revision and the session export contains the checked answer.
