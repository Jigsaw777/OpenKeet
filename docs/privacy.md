# How OpenKeet handles your data

This guide covers the Windows desktop preview. See [profile migration](legacy-python.md) if you used the archived Python app.

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

## Interview packs, facts and practice

- **Where.** Documents you add, the facts found in them, stories, interview packs, answers and practice history are stored in a separate local database, `prep.sqlite3`, in the app data folder. Nothing is uploaded.
- **What leaves your PC.** Nothing, unless you press one of these buttons. Each sends data only to the answer provider you connected:
  - **Draft with AI** or **Draft N answers with AI**: each question, its outline and your confirmed facts for that topic.
  - **Suggest facts**: the text of that one document, up to 24,000 characters.
  - **Get AI feedback**: the practice question, its key points and your answer.

  Prepared answers in sessions never call a provider.
- **Reworded-question matching.** Turning on **Catch reworded questions** sends your pack's question wordings, and later any live question wording alone can't match, to Ollama on this PC (`127.0.0.1`) only. The resulting vectors are cached in `prep.sqlite3`, only for the pack in use, and deleted with **Delete all my prep data** or when you turn the feature off.
- **Voice answers in Practice.** **Answer by voice** uses your microphone and the local speech model only while you're recording. The transcript is discarded unless you submit it.
- **Community packs.** Installing a pack copies public questions into a separate table. Exporting a shareable pack uses a field allowlist and never includes your documents, facts, stories, answers, names or file names. Pack links are never opened automatically.
- **Logs.** Prep features write no logs. Their in-memory change list holds identifiers only, never document, transcript or answer text.
- **Delete.** Remove a document in **Prepare**, a pack in **Manage this pack**, practice history in **Practice**, or everything with **Delete all my prep data** in **My experience**. **Save private backup** writes a JSON file you control; it contains your private data.

Feature details: [InstantPrep](features/instantprep.md), [Anticipate](features/anticipate.md), [TrueStory](features/truestory.md), [Challenge Mode](features/challenge-mode.md), [Community Packs](features/community-packs.md).
