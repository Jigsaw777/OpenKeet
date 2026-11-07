# Indie Hackers builder post

The Indie Hackers site and Products directory were visible. Check the current group's posting rules before using this draft or including a product link. Publish only after the public release is usable. [Indie Hackers](https://www.indiehackers.com/)

**Title.** A solo developer trade-off: local speech and user-chosen answers in OpenKeet

**Post**

I’m building OpenKeet, a Windows interview practice app. The product starts with a small task: pick one question, say what you think, and work on how to explain it.

I wanted a setup I could maintain as one developer. Speech recognition runs on the user's PC. An answer can come from Ollama on the same PC or from a supported service the user chooses. That means I don't need to run an inference server for the current desktop app. It also puts more work into the first-run experience. People need to understand which model they need and why there are separate audio and answer settings.

I’m making the source available so those choices can be inspected and improved. The first work I want help with is small: a clearer setup guide, keyboard testing, or a synthetic speech case that someone else can reproduce.

If you’ve shipped a desktop product with a local dependency, what helped people reach their first useful result? I’m especially interested in how you found out whether a person had a setup problem or had simply chosen a different path through the app.

Project and setup: [VERIFIED_GITHUB_REPOSITORY_URL]

**Suggested visual.** The real Windows setup screen with the Listen to, Speech model, and Answers steps visible.

**Follow-up.** “I plan to group first-run reports by the step where the user stopped. If you've tried that without tracking users in the app, what did you learn?”
