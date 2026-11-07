# Reddit launch drafts

These drafts are alternatives. Publish only after the owner approves the public release, the links work while signed out, and the live community rules allow the post. Disclose that you maintain OpenKeet. Use each subreddit at most once in the initial launch wave. Review any first-person claim before posting. [Research and rule sources](reddit-community-matrix.md) · [Placeholder checklist](launch-assets.md)

## 1. A free, open-source interview preparation tool

**Target and eligibility.** r/opensource. The sidebar allows some promotion and limits repetition. Use the current Promotional flair if required. The source and license must be public. [Rules and examples](https://www.reddit.com/r/opensource/)

**Title.** I built OpenKeet, an open-source Windows app for practicing interview answers

**Post**

Interviews can be stressful. Preparing for them should be easier to start. I built OpenKeet so you can take one question you expect to hear, practice an answer, and work through what you want to say in your own words.

The Windows preview lets you type a question with an answer model connected. You can also choose a microphone or playback device for a permitted practice call. Transcription happens on your PC. For answers, you can use a local Ollama model or choose a connected provider. Cloud providers receive the question and related text when the app requests an answer.

The project source is under MIT. I want people to be able to inspect the data flow, try it on their own hardware, and improve it. The public repo includes a Windows setup guide, privacy notes, and ways to contribute.

If you try it, I'd like to know where you first got stuck. Was it choosing a device, setting up a model, or figuring out what to ask?

Source and setup: [VERIFIED_GITHUB_REPOSITORY_URL]

**Media and link.** One real setup screenshot or a captioned demo if the composer accepts it. Use one direct repo link. Audience: users and contributors.

**First comment.** “Start with one typed question if you already have an answer model. If you'd rather check the audio path first, choose Transcription only and try a sentence you wrote yourself.”

**Likely question.** “Is it free?” The app has no subscription. Downloaded local models have their own requirements. Connected providers may bill separately.

## 2. Local transcription and Ollama

**Target and eligibility.** r/ollama. Comparable Windows app posts are visible. Confirm the current project-post rules before submitting. For r/LocalLLaMA, write your own submission and check whether both the post and any linked code meet its limits on LLM-generated material. [r/ollama](https://www.reddit.com/r/ollama/) · [r/LocalLLaMA rules](https://www.reddit.com/r/LocalLLaMA/)

**Title.** How would you test a local Ollama setup for interview practice on Windows?

**Post**

I'm the maintainer of OpenKeet, a Windows interview practice app. I kept speech recognition and answer generation separate. The app can use local Whisper or experimental NVIDIA Parakeet to turn audio into text. If I select a downloaded model in Ollama, OpenKeet sends the question to Ollama on the same machine. I can also switch answers off and use transcription on its own.

The smallest check is to start Ollama, download a local model, select it in OpenKeet, and ask “When would you use an LRU cache?” No speech setup is needed for that check. Then try a short follow-up and see whether the suggestion stays on topic.

I'd value one reproducible report from a different Windows setup. Which local model did you use, what hardware was available, and which step caused friction? A small example is more useful than an overall speed rating.

Source and setup: [VERIFIED_GITHUB_REPOSITORY_URL]

**Media and link.** Use the local provider selector and a synthetic question. Include one repo link if allowed. Audience: Ollama users and local inference developers.

**First comment.** “The checked configuration is local Ollama on the same PC. Please include the model name and whether the problem happened when listing models or generating a reply.”

**Likely question.** “Does it have a local answer cache?” The current preview requests answers from the selected model. Cached or precomputed answers are outside this release.

## 3. A desktop client for people already running Ollama

**Target and eligibility.** r/selfhosted. Use the current New Project Megathread only if the project qualifies under its age and content rules. Ask moderators whether this Windows companion fits. A standalone Wednesday post needs the relevant tool flair and the current readiness requirements. [Rules and exceptions](https://www.reddit.com/r/selfhosted/)

**Title or thread label.** OpenKeet: a Windows interview practice client for a local Ollama setup

**Post**

I maintain OpenKeet, a Windows desktop app for practicing interview answers. For people already running Ollama on a Windows PC, here's the integration: the app can transcribe the selected mic or playback device locally, and a question can go to a downloaded Ollama model on that same PC. Completed sessions are saved to a local library only when you choose to save them.

I took this route so people could use their own hardware and choose an answer provider. OpenKeet itself is a desktop client. If you turn on a cloud provider, the selected service receives the question and relevant text for each answer request.

I'd like feedback on where the documentation for a local setup could be clearer. Is the distinction between the speech model and Ollama's answer model clear before you download either?

Source and setup: [VERIFIED_GITHUB_REPOSITORY_URL]

**Media and link.** Text comment with one source link only when the live thread permits it. Audience: local-service maintainers. A video is optional only for an approved showcase.

**First comment.** In a thread, keep the explanation in the comment. Reply with the relevant setup step when asked.

**Likely question.** “Can I point the app at a separate Ollama server?” The documented route uses Ollama on the same PC. Treat remote configuration as a feature discussion.

## 4. A desktop engineering problem worth discussing

**Target and eligibility.** r/tauri. Technical showcase posts are visible, but confirm the current project-link rules before posting. [Community and examples](https://www.reddit.com/r/tauri/)

**Title.** Testing a Windows speech-to-answer pipeline in a Tauri 2 app

**Post**

I built OpenKeet in Tauri 2, Rust, and React. One problem I wanted to make reproducible was what happens when the spoken question changes while an answer is already coming back.

The Windows audio layer captures one chosen mic or playback device. A local decoder produces text. The Rust session tracks question revisions, so a later correction can supersede an earlier suggestion. The WebView gets session updates separately from the audio work. A synthetic audio fixture and local test provider let us exercise the path without sharing a real interview or needing a cloud account.

I'm looking for Windows failure cases we can turn into regression tests. If you have shipped a similar desktop app, which case would you cover first: an audio device switching mid-session, a short follow-up after a pause, or recovering the window with the keyboard?

Source and test notes: [VERIFIED_GITHUB_REPOSITORY_URL]

**Media and link.** One current screen or a short synthetic fixture recording. Use a source link as context for the engineering question. Audience: desktop engineers.

**First comment.** “The Windows smoke path can use a known WAV and a local HTTP provider fixture. A reproducible device transition or short WAV is the most useful kind of report.”

**Likely question.** “Why Windows only?” The packaged and tested preview is Windows 11 x64, with WASAPI capture. Other platforms need their own capture and release checks.

## 5. One practice exercise for a tech candidate

**Target and eligibility.** r/cscareerquestions, in the current monthly promotion sticky only. Rule 5 sends promotions there. This is a thread comment. [Sidebar rules](https://www.reddit.com/r/cscareerquestions/)

**Thread label.** A Windows app for practicing technical follow-ups

**Post**

I'm building OpenKeet, a Windows interview practice app. One exercise I find useful is to answer a question once, then test the answer against a follow-up. Try “When would you use an LRU cache?” Explain your decision aloud. Then ask “What would change if the data became stale?” and see which assumption you left out.

You can do the exercise on paper or try it with OpenKeet. The app accepts typed questions with a connected answer model, and it can transcribe permitted practice audio. A local speech model handles transcription on your PC. The app can use local Ollama for suggestions after setup.

If you try it, which part of the exercise takes the most practice: starting the explanation, naming a trade-off, or handling the follow-up?

Setup: [VERIFIED_RELEASE_DOWNLOAD_URL]

**Media and link.** Text and one verified release link if the thread permits it. Audience: candidates preparing for technical rounds.

**First comment.** Keep the exercise in the main thread comment. Reply to follow-up questions in that thread.

**Likely question.** “Can I use it in an assessment?” Only where the rules allow AI assistance and everyone has given permission for audio capture. The exercise works in solo practice.

## 6. Feedback from developers preparing for technical rounds

**Target and eligibility.** r/developersIndia, in the live Showcase Sunday thread if project links are allowed there. The community calendar lists it monthly. [Sidebar and current thread](https://www.reddit.com/r/developersIndia/)

**Thread label.** OpenKeet: would this first-run flow help you practice a technical answer?

**Post**

I maintain OpenKeet, a Windows desktop app for interview preparation. The workflow I'd like feedback on is simple: pick one question, answer it yourself, and then see whether a suggestion helps you explain your reasoning more clearly.

For a technical round, try asking how you would handle retries in a service that receives the same request twice. If you prefer behavioral practice, try explaining one trade-off you made on a project. You can type the question with an answer model connected. The audio option can transcribe a practice call on your PC when everyone agrees.

I'd especially value feedback on getting to a first useful answer. If you try it on Windows, where did setup stop or get confusing? A short description of the step is enough.

Source and Windows setup: [VERIFIED_GITHUB_REPOSITORY_URL]

**Media and link.** One link in the approved thread. Add a screenshot only if permitted. Audience: India-based software developers and candidates.

**First comment.** “If you report an audio issue, please include the Windows version and whether you selected a mic or playback device. Synthetic audio is enough.”

**Likely question.** “How well does it handle my accent?” The project has synthetic voice comparisons, including Indian English voices. That evidence does not establish accuracy on your microphone or calls.

## 7. A solo builder lesson about first-run setup

**Target and eligibility.** r/SideProject. Check the general feed's current AI and self-promotion rules. The highlighted non-AI megathread is for other projects. [Community](https://www.reddit.com/r/SideProject/)

**Title.** My interview practice app starts with one question. Is the setup clear enough?

**Post**

I'm building OpenKeet, a Windows app for practicing interview answers. There are two different setup tasks: getting speech into text and getting a model to suggest an answer. Putting both in front of someone who just wants to try one question can make the app feel more complicated than it is.

The current flow offers two starts. If you want to type a question, connect an answer model and ask it. If you want to practice with audio, choose one input and download a speech model. You can use that second path with transcription only.

I'd like to make the first choice obvious. If you looked at the screenshot, would you know which route to pick? If you've built a tool with optional setup, what helped people get to their first useful result?

OpenKeet and the setup guide: [VERIFIED_GITHUB_REPOSITORY_URL]

**Media and link.** Attach the current first-run screenshot and one repo link if allowed. Audience: makers who can critique onboarding.

**First comment.** “The screenshot is from the Windows app. The speech model and the answer provider can be configured separately.”

**Likely question.** “Why not run everything in a hosted app?” The current approach lets people run speech recognition locally and decide where answer generation happens. Setup feedback will help make that choice clearer.

## 8. How can contributors get started?

**Target and eligibility.** r/indiehackers. The sidebar allows one product post with Self Promotion flair for feedback and critique. Use this instead of a separate launch post there. [Sidebar rules](https://www.reddit.com/r/indiehackers/)

**Title.** What would make you comfortable opening a first PR for a small Windows app?

**Post**

I maintain OpenKeet, an open-source Windows interview practice app. I'm trying to make it easy to help without asking someone to understand the whole audio stack first.

A few concrete starting points are checking the first-run guide against a fresh setup, trying keyboard navigation on the Settings screen, and adding a small synthetic question fixture that captures a failure. The contribution guide explains how to open a focused PR and report what you verified.

If you've contributed to a project this size, what makes a first task feel doable? A clear expected result, a command to reproduce it, or a maintainer reply that helps you choose a smaller scope?

If one of those areas interests you, I'd welcome a focused improvement. These are volunteer contributions.

Contribution guide: [VERIFIED_CONTRIBUTING_URL]

**Media and link.** One link to the guide. A screenshot of the relevant screen is optional if allowed. Audience: independent builders and potential contributors.

**First comment.** “A documentation report with the step, Windows version, and expected result is useful even if you never open a code PR.”

**Likely question.** “Where should I begin if I don't know Rust?” Try the Windows guide and point out one confusing step. That can become a small docs change.

## Optional independent interview-prep discussion

Use only if the current rules allow this kind of advice in r/InterviewPrep or r/interviews. This exercise stands on its own. Keep it free of product links and follow-up promotion. [r/InterviewPrep](https://www.reddit.com/r/InterviewPrep/) · [r/interviews](https://www.reddit.com/r/interviews/)

**Title.** A two-pass exercise for a behavioral interview answer

**Post**

Pick one real decision you made at work or in a project. Set a 90-second timer. Explain what was happening, what you were responsible for, what you did, and how it turned out. Stop when the timer ends.

Before trying again, write one sentence about a choice you made and why. Keep that sentence. Cut one detail that the listener doesn't need. Now answer the same question again without looking at the first version.

A useful follow-up is “What would you do differently?” That can show where your first answer described the outcome but skipped the reasoning.

If you use this exercise, which part is hardest to get into the answer without making it longer: your role, the reason for your choice, or the result?
