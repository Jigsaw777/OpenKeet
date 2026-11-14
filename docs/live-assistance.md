# Live interview assistance

OpenKeet follows a live interview as it happens. It transcribes the selected audio, flags likely questions, and shows answer suggestions in Focus when an answer model is connected. Get permission before processing someone else's audio.

1. Select one source under **Listen to**. Choose your microphone or the Windows playback device carrying the call. With a playback device selected, **Also transcribe my microphone** adds your own side: those lines are labelled **You** and are never read as a question. Wear headphones for it. Through speakers the microphone hears the call too, and lines that overlap the call's speech are dropped as echo.
2. Set up the **Speech model**. If you connect an [answer model](ai-providers.md), review **Answer detected questions automatically** before you start. Turn it off if you want to request each suggestion yourself.
3. Select **Start listening**. The Focus window shows the listening status, detected question, and current answer. While someone is speaking, the status line shows the words heard so far and updates as they continue. Select **Get answer** to request a suggestion. If a question is missed, try **Answer what they said** or type it in the main window and press **Enter**.
4. Use **Pause** or **Stop** when appropriate. Hiding the window pauses listening. Bring it back through the tray icon or **Ctrl+Alt+I**, then select **Resume**. A paused session keeps its speech model in memory for 10 minutes, so resuming within that time starts at once. After that, or after **Stop**, the model is released and loads again on the next start.
5. If the audio device drops out mid-session, OpenKeet reopens it up to three times and then switches to the Windows default device of the same kind. A notice says audio was interrupted. If that also fails, the session stops with **Retry same source**. A microphone failure only ends the **You** lines.

Question detection can miss short follow-ups or misread a statement as a question. Check the transcript and the current question before relying on a suggestion. Select **Transcription only** for a session without AI answers.

After stopping, save to **History** only if you want to keep the retained text. [What OpenKeet processes and stores](privacy.md)

## Screen-share privacy on Windows

1. Open **Settings**, then **Audio and display**.
2. Turn on **Screen-share privacy**. It applies to the workspace and the Focus overlay until you turn it off or quit OpenKeet.
3. Check the receiving view in your screen-sharing app before relying on the setting.

The setting starts off each time you launch OpenKeet. It uses Windows display affinity, which applies to compatible capture methods. [Microsoft's display affinity documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity)
