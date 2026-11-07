# How OpenKeet handles your data

This guide covers the Windows desktop preview. The older Python prototype has [separate behavior](legacy-python.md).

## When you start listening

You choose one microphone or one Windows playback device. Windows playback capture can include any sounds playing on that device. Use a consented session and stop unrelated playback. Windows microphone permissions apply when you choose a mic.

A local speech model processes audio in memory. The app keeps a bounded working transcript and answer history for the session. Use **Pause**, **Stop**, or **Clear current session** to control the active workspace.

## What goes to an answer provider

- **Transcription only** leaves the answer route off.
- **Ollama** receives text through the service on your PC. Choose an installed local model.
- **ChatGPT or Claude Console** receives the question or text you submit and linked question-and-answer context when applicable. If you use an action that asks about transcript text, that text is included. Audio stays with the local speech decoder.

The provider notice appears before you activate a cloud route. If automatic answers are enabled, detected questions can trigger these requests. Provider terms and account controls apply to text sent to cloud services. Switching routes or disconnecting affects future requests. [Provider setup](ai-providers.md)

## What stays on your PC

- **History.** Select **Save to history** after stopping to save retained final transcript lines and checked answers. The Windows app stores these and any notes in a local SQLite database (`sessions.sqlite3`). Windows file permissions apply. The database is readable as text by software that can access it.
- **Exports.** Markdown, JSON, and TXT exports are separate files. Delete those files or any backups yourself if you no longer need them.
- **Connections.** Windows protects saved provider credentials with DPAPI, the operating system's per-user data protection. Use **Settings**, then **Answers**, to sign out of ChatGPT or remove a Claude key.
- **Models and settings.** Downloaded speech weights and app settings are stored locally. Ollama manages its own downloaded models.

Use **History** to delete saved sessions. This removes the entry from the app library. Use **Clear current session** to reset the workspace. [Technical data boundaries](architecture/DESKTOP_3_0.md)
