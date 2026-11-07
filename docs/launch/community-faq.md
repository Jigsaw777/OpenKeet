# OpenKeet community FAQ

Use these answers in launch threads after replacing links and checking the release tag.

**What is OpenKeet?** OpenKeet is your personal AI interview assistant for Windows. Practice a typed question, transcribe permitted practice audio, and work through suggestions in your own words. [VERIFIED_GITHUB_REPOSITORY_URL]

**Where do I get it?** Open the verified Windows x64 release and follow its setup steps. The release notes give the signing status and checksum. [VERIFIED_RELEASE_DOWNLOAD_URL]

**What works without an internet connection?** After model downloads, local Whisper or experimental NVIDIA Parakeet can transcribe audio on your PC. A downloaded Ollama model can generate answers locally. The build, model downloads, and cloud answer routes need internet access.

**Do I need to pay?** OpenKeet has no app subscription. Local transcription and local Ollama need no AI service account. Claude Console uses your API key and workspace billing. Preview ChatGPT sign-in depends on eligibility and available usage.

**What does OpenKeet send outside my PC?** Audio is processed locally. If you choose a cloud provider, answer requests send question text and relevant context to that provider. Automatic suggestions can also send these requests. Choose **Transcription only** or a local Ollama model for local processing.

**What is saved?** A completed session enters the local library when you choose **Save to history**. Review, export, or delete it under **History**. Exports are separate files you manage.

**Can it read my résumé or run a mock interview for me?** The Windows preview works with questions you type and speech you choose to capture. Résumé and job-description grounding, precomputed answers, adaptive mock interviews, and shared packs are outside the current preview.

**What operating systems are supported?** The packaged preview targets Windows 11 x64. The older Python prototype is a separate source path. Report OS and build details if setup fails.

**Can I use it during an interview?** Use audio or AI assistance only where interview and assessment rules allow it. Get consent before capturing other people's audio. Practice on your own if permission is unclear.

**How do I help?** Try one workflow and share a specific step that worked or got in your way. Developers can start with documentation, keyboard navigation, or a synthetic speech fixture. [VERIFIED_CONTRIBUTING_URL]

**Where do I report a problem?** Use Discussions for setup questions and an Issue for a reproducible bug. Follow the security policy for vulnerabilities. Use synthetic examples, and remove personal transcripts, account details, and API keys. [VERIFIED_DISCUSSIONS_URL]
