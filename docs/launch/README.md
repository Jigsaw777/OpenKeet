# OpenKeet launch library

**OpenKeet — Your Personal AI Interview Assistant.**

**Interview smarter. Answer better.**

Use this library to prepare the private Windows preview for a controlled public launch. Check the [release gates](../OPEN_SOURCE_RELEASE_CHECKLIST.md) before sharing a draft. Start with the [migration report](repository-migration.md) and [90-day plan](90-day-growth-plan.md).

| Work | Guide |
| --- | --- |
| Pick channels | [Platform research](platform-research.md) and [Reddit matrix](reddit-community-matrix.md) |
| Write a community post | [Reddit drafts](reddit-posts.md), [LinkedIn drafts](linkedin-posts.md), [Hacker News prep](hacker-news.md) |
| Prepare a launch or article | [Product Hunt](product-hunt.md), [DEV Community](dev-community-article.md), [Indie Hackers](indie-hackers.md) |
| Show the app | [YouTube scripts](youtube-scripts.md) and [launch assets](launch-assets.md) |
| Check claims and publish when approved | [Community FAQ](community-faq.md) and [launch checklist](launch-checklist.md) |

## What the Windows preview can show

- Type your own question with an answer provider connected. Use the suggestion to check your reasoning, then say it in your own words.
- Capture one Windows playback device or microphone with permission. Local Whisper or experimental NVIDIA Parakeet transcribes the audio. Question detection can update its interpretation as speech settles. [Speech setup](../getting-started.md#3-practice-with-audio)
- Choose transcription only, local Ollama, Claude Console with your own API key, or the preview sign-in for eligible ChatGPT accounts. A cloud provider receives question text and relevant context when an answer is requested. [Provider guide](../ai-providers.md)
- Save completed sessions explicitly to local history. Search, add notes, export, or delete them. [Privacy guide](../privacy.md)

The reviewed `main` preview has no shipped résumé or job-description grounding, precomputed answers, adaptive mock interviews, personal experience store, or contributed interview pack workflow. Predicting the answer or question before the person asks is separate from the implemented question detection. Keep those ideas in roadmap or feedback language. The older Python prototype has a separate scope.

## Match the message to the reader

| Audience | Main problem | Promise | Proof to show | Ask |
| --- | --- | --- | --- | --- |
| Students | First interviews feel unfamiliar | Practice one question in your own words | Type a first question and compare the suggestion | Name one confusing setup step |
| Job seekers | Preparation feels scattered | Turn a hard question into something you can rehearse | A typed question followed by a saved note | Share the question type you struggle with |
| Software engineers | Explaining trade-offs takes practice | Work through a technical answer and follow-up | A cache invalidation or system-design example | Try a technical question and report where the answer loses context |
| Experienced professionals | Leadership stories get too long | Practice a concise example based on real experience | One situation, action, and result | Share which part is hardest to explain |
| Local AI enthusiasts | Data flow and local setup are unclear | Choose where speech and answers are processed | Local speech and a downloaded Ollama model | Share model, hardware, and a reproducible setup result |
| Open-source developers | Contributions are hard to start | Take a small task with a real user impact | A tagged issue and reproducible check | Pick a documentation or test issue |
| Desktop app developers | Audio devices and native state are tricky | Compare a real Windows audio path and UI | A synthetic loopback clip or resize and keyboard demo | Suggest a focused fixture or platform fix |
| Technical creators | Demo claims are hard to substantiate | Show a workflow from a verified build | A captioned clip with build, provider, and OS stated | Reproduce the demo and link the tested release |
