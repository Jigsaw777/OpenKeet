# OpenKeet recording scripts

Record the approved Windows release with a clean profile and synthetic questions. Show the Windows version, OpenKeet tag, and selected provider. Use captions. In any audio scene, use your own scripted voice or a consenting participant and stop unrelated playback. Review every frame for account information.

For a Short, use a related walkthrough or channel link. YouTube makes URLs in Shorts descriptions and comments non-clickable. Put the verified release link in the long-form description where external links are enabled. [YouTube link formats](https://support.google.com/youtube/answer/13748639)

## 1. A 45–60 second introduction

**Title.** Practice one interview question with OpenKeet

| Time | Record and caption | Voiceover |
| --- | --- | --- |
| 0–8s | Title card. Caption: “Interview smarter. Answer better.” | Interviews are stressful enough. Preparing for them shouldn't be. OpenKeet is a personal AI interview assistant for Windows. |
| 8–22s | Show **Session**, then **Ask a question** with a local answer provider already connected. | Start with a question you expect to hear. I’m using “When would you use an LRU cache?” Answer it yourself first, then read the suggestion. |
| 22–36s | Type the follow-up. Caption: “Practice the follow-up.” | Now ask what changes if the data becomes stale. That is where you find out whether you explained your assumptions. |
| 36–48s | Show **Speech model** and **Answers** as separate options. | If you practice with audio, OpenKeet can transcribe on your PC. You choose where answers are generated. |
| 48–57s | Closing card with verified project name and related video prompt. | Watch the setup walkthrough to try your first question on Windows. |

## 2. A three-minute walkthrough

**Title.** OpenKeet on Windows: setup, your first question, and a follow-up

| Time | Record and caption | Voiceover |
| --- | --- | --- |
| 0:00–0:20 | First-run screen. Caption: “Windows 11 x64 preview.” | OpenKeet gives you two ways to start. You can ask a typed question after connecting an answer model, or set up audio for a practice session. I’ll show the typed path first so you can see one result before deciding whether you need a speech model. |
| 0:20–0:50 | Open **Settings**, then **Answers**. Show **Ollama** and a downloaded local model. | I already have Ollama running with a downloaded local model. I select it and use it for answers. If your model list is empty, start Ollama, download a local model, and refresh the list. The provider guide also explains which connected services are supported and what they charge for. |
| 0:50–1:25 | Return to **Session** and open **Ask a question**. Type the question, then show a genuine reply. | Here’s the question: “When would you use an LRU cache?” Before reading the suggestion, I’d say that the cache can keep recently used items close at hand, but it needs a size limit and a plan for stale data. Compare your explanation with what the model says. Keep what you can explain. |
| 1:25–1:55 | Type “What changes if the data becomes stale?” Scroll through the actual follow-up. | For the follow-up, think about invalidation, expiry, and how stale a response can be. A suggestion can help you spot a missing assumption. It is still your reasoning that you want to practice. |
| 1:55–2:32 | Show **Listen to** and **Speech model**. If recording audio, play the synthetic clip through the selected output. | For audio practice, select one mic or playback device. Download the local speech model. Get permission before starting. Use transcription only if you just want text. When a cloud answer provider is selected, its answer requests receive the question and relevant text. |
| 2:32–3:00 | End the session, save it if useful, and show **History**. | Choose Save to history only if you want a local copy. You can add notes, export it, or delete it in History. The description has the release and setup guide. If setup stops you, tell me which step and which Windows version you were using. |

**Long-form description.** Practice one interview question on Windows. Release and setup: [VERIFIED_RELEASE_DOWNLOAD_URL]. Source, privacy guide, and reporting: [VERIFIED_GITHUB_REPOSITORY_URL].

## 3. Technical architecture demonstration

**Title.** How OpenKeet keeps speech, question state, and answers separate

| Time | Record and caption | Voiceover |
| --- | --- | --- |
| 0:00–0:30 | Show three simple boxes labeled “Windows audio,” “Question state,” and “Answer provider.” | The question I wanted to answer was what happens when an interviewer changes the question halfway through. OpenKeet separates getting text, deciding which question the session is on, and asking a model for a suggestion. That makes each boundary easier to test. |
| 0:30–1:10 | Show **Listen to** and **Speech engine**. Play a known synthetic WAV. | On Windows the app reads one chosen microphone or playback device. It segments speech locally and decodes it with Whisper or experimental Parakeet. Watch the transcript as a short question arrives. The audio stays with the local speech path. |
| 1:10–1:55 | Show session code or a test using a later correction. Caption: “A newer question can supersede an older answer.” | A partial line can change, and a correction can change what the answer should address. The Rust session tracks question revisions. A fixture can exercise that transition with the same text and timing each time. This is where I look when a late result no longer matches the current question. |
| 1:55–2:40 | Show the answer selector and local provider fixture or actual local Ollama route. | The provider handles generation separately. With local Ollama, the selected model runs on this PC. With a cloud provider selected, question text and relevant context go to that provider when an answer is requested. A local fixture can exercise the streaming path without using a real account. |
| 2:40–3:30 | Show the actual benchmark table, including the worse telephone-band result. | On 116 synthetic clips, Whisper and Parakeet recorded 18.02 and 13.33 percent word error rates in the paired live pipeline run. On the simulated telephone subset, Parakeet had more errors. The comparison uses one PC and synthetic speech. It doesn't tell us how a real meeting or your mic will perform. |
| 3:30–4:05 | Open contributor guide and highlight one test or guide area. | To help, start with a small reproduction: a short follow-up, a correction across a pause, or a device-change issue. Use a synthetic or consented example and include the build and device route. The source and benchmark method are linked below. |

**Technical description.** Code and setup: [VERIFIED_GITHUB_REPOSITORY_URL]. Matched synthetic speech results and limitations: [VERIFIED_BENCHMARK_URL]. Contribution guide: [VERIFIED_CONTRIBUTING_URL].

Use the [launch assets](launch-assets.md) checklist to verify the final links, captions, and permissions.
