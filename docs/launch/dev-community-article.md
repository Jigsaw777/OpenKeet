# Building OpenKeet: A Local-First, Open-Source AI Interview Assistant

**Publication note.** Review this draft against the final public tag, replace the links, and keep the AI disclosure at the end. Suggested tags are `rust`, `tauri`, `opensource`, and `ai`. [DEV article guidelines](https://dev.to/guidelines-for-ai-assisted-articles-on-dev/)

## Start with a question

Interviews can be stressful. Preparing for them shouldn't require a hosted workflow just to practice an explanation. I built OpenKeet so I could take one question, answer it, and work through the follow-up.

A simple example is “When would you use an LRU cache?” I want to explain my assumptions before I read a suggestion. Then I can ask what changes if the cached data becomes stale. I can do that with a typed question or use local transcription for a practice call where everyone has agreed to it.

The project is a Windows desktop app. The current package targets Windows 11 x64. Project source: [VERIFIED_GITHUB_REPOSITORY_URL]

## Three jobs that should stay separate

**Get speech into text.** A Windows WASAPI input or playback route provides the audio. The app segments voiced speech and decodes it on the PC. Whisper small.en is the default. The preview also offers experimental NVIDIA Parakeet v3 through sherpa-onnx. Users download the model they choose.

**Figure out what the question is.** The Rust session receives transcript events, keeps a working transcript, and tracks question state. A partial line can change as speech continues. The system needs to wait for enough evidence and revise its understanding when the speaker corrects a term or adds a constraint.

**Generate an answer if requested.** An answer model is a separate choice. Ollama on the same PC is the local route. There is also Claude Console with a user-supplied API key and preview sign-in for eligible ChatGPT accounts. Cloud answer requests include the question and relevant text context. A person can choose transcription only.

Tauri connects the React interface to the Rust backend. The audio path and session state stay on the native side. The UI receives updates for status, questions, and answers. When someone explicitly saves a completed session, it goes into a local SQLite library.

This separation lets someone try a typed question before downloading a speech model. It also makes the privacy decision easier to explain.

## A partial transcript is a moving target

Consider “When would you use a cache?” followed by “I mean a cache with a fixed memory budget.” The early text is helpful to display, but an answer based on the first interpretation can become stale. A long pause can also look like the end of a question when it is only a pause.

OpenKeet tracks question revisions so a newer interpretation can supersede older answer work. The interface also distinguishes the transcript, detected question, and answer status. That helps a user see what the app heard and decide when to request a suggestion.

The tests are important here. A synthetic recording lets us reproduce the same words and pauses. A local test provider can exercise streaming and a retry path without requiring a cloud account. The tests cover state transitions that are easy to miss when the UI looks correct after everything has finished.

## What changed when I compared speech engines

I used the same paced audio pipeline, segmentation, and 116 synthetic clips, including four non-speech controls. Both runs used one Windows 11 x64 PC with an Intel i5-1335U and 16 GiB RAM.

| Result on this run | Whisper small.en | Parakeet v3 INT8 |
| --- | ---: | ---: |
| Word error rate across the pack | 18.02% | 13.33% |
| Final transcript latency, median on 112 spoken clips | 1.353 s | 1.090 s |
| Word error rate on simulated telephone-band technical words | 26.79% | 40.18% |

The aggregate result favored Parakeet. The telephone-band result favored Whisper. Parakeet also missed more words when a speaker corrected the question across a pause in this corpus. That is why Whisper remains the default and Parakeet is experimental.

The run is a comparison of synthetic speech on one machine. It doesn't establish quality for real interviews, meeting codecs, microphones, or another PC. Reproduction steps, per-clip results, and limitations: [VERIFIED_BENCHMARK_URL]

## Try one step or improve one thing

Start with the Windows setup guide. With an answer model connected, ask a typed question. If you want to test audio separately, choose transcription only and use your own short synthetic practice clip. Save the session only if you want to keep it.

Three useful contributions are a correction to a setup step, a keyboard-navigation report, or a small speech fixture that reproduces a missed question. Use synthetic text and leave account details out of reports. The project does not yet ship résumé grounding, adaptive mock interviews, or precomputed interview packs.

I would particularly like feedback on a narrow case: what short correction or follow-up is most likely to change how you would answer the original question?

Contribution guide: [VERIFIED_CONTRIBUTING_URL]

*Disclosure: I used AI assistance to draft this article. I reviewed the implementation and the measured results before publishing.*
