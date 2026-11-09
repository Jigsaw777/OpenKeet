<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/assets/openkeet-logo-on-dark.svg">
    <img src="docs/brand/assets/openkeet-logo-on-light.svg" alt="OpenKeet" width="430">
  </picture>
</p>

# OpenKeet

**Your personal AI interview assistant.**

**Interview smarter. Answer better.**

OpenKeet is a real-time interview copilot for Windows. It transcribes speech on your PC, flags likely questions, and offers live answer suggestions. Build interview packs from your own experience, rehearse likely questions, and bring prepared answers into sessions that allow live AI assistance.

[Windows setup downloads](setup/README.md) · [See it in action](#see-it-in-action) · [Documentation](docs/README.md) · [Contribute](CONTRIBUTING.md)

**Windows 11 x64 preview.** Download and run the setup file, then open OpenKeet from Start. [First run](docs/getting-started.md) · [Build from source](docs/getting-started.md#1-build-the-windows-preview) · [MIT source license](LICENSE)

The desktop backend and command-line tools use Rust. If you used the older Python prototype, [import your saved profile](docs/legacy-python.md).

## What you can practice

| If you want to… | Try this in OpenKeet |
| --- | --- |
| Explain a technical idea | Ask about a coding concept or a design trade-off. Practice explaining the answer aloud. |
| Tell a clearer story | Ask for feedback on a behavioral answer. Give the situation, your task, your action, and the result. |
| Follow a practice call | Select the call's audio or your microphone. OpenKeet transcribes locally and flags likely questions. |
| Work through a follow-up | Request a suggestion for the detected question or ask your own. Save the session if you want to revisit it. |

Typing one question you expect to hear is a good place to start if you're preparing for your first interview. Engineers can work through trade-offs, career switchers can practice describing transferable skills, and experienced candidates can rehearse leadership stories. [Practice guide](docs/interview-preparation.md)

## Prepare for the questions that matter

Add your resume and the role you're applying for. OpenKeet helps you organize topics, practice tougher follow-ups, and review answers grounded in your own experience. Use local AI or a supported provider you choose. Community-created interview packs make it easy to learn from others.

| Feature | What it does | Status |
| --- | --- | --- |
| [Interview packs](docs/features/instantprep.md) | Resume + job description → ranked topics, questions, and 15 s / 45 s / 2 min answer drafts | Available offline. AI drafts need a connected provider |
| [My experience](docs/features/truestory.md) | Facts from your documents with sources; confirm, correct, reject; conflict checks | Available offline |
| [Practice](docs/features/challenge-mode.md) | Adaptive follow-ups, transparent feedback, readiness map | Type offline or use voice with a local speech model |
| [Prepared answers in sessions](docs/features/anticipate.md) | Show approved matches for the current question revision without an AI call | Available |
| [Community packs](docs/features/community-packs.md) | Install, update and share data-only question packs | Available offline. No online catalog yet |

PDF import, optional local embedding search, AI-suggested facts and AI practice feedback are included. AI help runs only when you press it, with the provider you chose.

## From a question to a clearer answer

1. To start with a typed question, connect an answer model in **Settings**. Return to **Session** and open **Ask a question**.
2. To work from audio, choose what to listen to and download a local speech model. OpenKeet transcribes on your PC and flags likely questions.
3. Review the question and any suggested answer. Check the reasoning and say the answer in your own words.
4. When the session ends, choose **Save to history** if you want to keep the transcript and checked answers.

## See it in action

![OpenKeet start screen with the Listen to, Speech model and Answers steps](docs/assets/first-run.png)

This is the Windows app at first launch. [Screenshot gallery](docs/community/screenshots.md)

## Use AI your way

- **Transcribe on your PC.** After a one-time speech model download, transcription works offline without an account. [Speech setup](docs/getting-started.md#3-practice-with-audio)
- **Run answers on your PC.** Connect a local model through Ollama. Download size, memory use, and speed depend on the model and computer.
- **Connect a service.** The preview implements a Claude Console API-key route with separate usage billing and an experimental **Sign in with ChatGPT** route for eligible plan usage. Account access and usage limits depend on the provider.

[Model setup, eligibility, and billing](docs/ai-providers.md)

## Your interview prep is personal

You choose one microphone or Windows playback device when you start listening. OpenKeet transcribes that audio on your PC and keeps the current transcript in the session. If you select a cloud provider, each answer request sends the question and relevant text to that service. This includes automatic suggestions when enabled. Save a finished session only if you want a local history entry. You can export or delete it from **History**. [Data and privacy guide](docs/privacy.md)

For real interviews or assessments, follow the interviewer's rules. Get permission before capturing someone else's audio or using live AI help. [Live assistance guide](docs/live-assistance.md) · [Screen-share privacy on Windows](docs/live-assistance.md#screen-share-privacy-on-windows)

## Common questions

**Is it free? Do I need an AI subscription?** OpenKeet has no app subscription. Local Ollama models need no AI service account. Connected services may charge for usage. The repository declares MIT for project source. The public distribution review is tracked in [release readiness](docs/release-readiness.md).

**Can I install it without programming?** Download the Windows x64 setup file and run it. [Setup snapshots](setup/README.md)

**Do I need a powerful PC or internet access?** Building the app and downloading models need internet access. Local transcription and local Ollama answers can work offline after setup. Connected answer services need internet access. Larger models can use more disk space and memory. [Model requirements](docs/ai-providers.md#local-answers-with-ollama)

**Can this help with a coding interview?** Use it to practice explaining code, choosing data structures, or describing system-design trade-offs. Review suggestions yourself. Your preparation and the interview determine the outcome.

**Does OpenKeet record interviews?** Listening processes audio in memory. The desktop app lets you explicitly save retained text and answers to a local library. [What gets saved](docs/privacy.md#what-stays-on-your-pc)

## Help shape OpenKeet

You don't have to write code to help. Share a confusing setup step, a question the app missed, or an accessibility issue. If OpenKeet helps you prepare, you can star the repo to find it again.

- [Ask a question or share an idea](https://github.com/Jigsaw777/OpenKeet/discussions)
- [Report a bug, request a feature, or flag a documentation gap](https://github.com/Jigsaw777/OpenKeet/issues/new/choose)
- [Find beginner-friendly issues](https://github.com/Jigsaw777/OpenKeet/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22)
- [Contribute code, testing, or docs](CONTRIBUTING.md)
- [See what's next](docs/roadmap.md)
