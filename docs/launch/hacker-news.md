# Show HN preparation

HN requires work people can try. It also bars generated text in submissions and generated or AI-edited comments. The material below is a private rehearsal of the facts. Write the submission and first comment yourself without AI editing. Verify the links while signed out, then stay available to respond. [Show HN guidelines](https://news.ycombinator.com/showhn.html) · [HN guidelines](https://news.ycombinator.com/newsguidelines.html)

**Headline to consider.** Show HN: OpenKeet, an open-source personal AI interview assistant for Windows

**Destination.** [VERIFIED_GITHUB_REPOSITORY_URL] or [VERIFIED_RELEASE_DOWNLOAD_URL], whichever gives a new visitor the clearest way to try it without gated signup. The repo needs a verified build or download, setup steps, and the exact limitations.

## Private first-comment rehearsal

I built OpenKeet because I wanted a way to practice explaining an interview answer and then see how it holds up to a follow-up. The Windows desktop app accepts a typed question with a provider connected. It also transcribes a selected microphone or playback device locally for a permitted practice session.

The project separates speech recognition, question state, and answer generation. Rust handles Windows audio and tracks revisions as the transcript settles. The app can use Whisper or experimental NVIDIA Parakeet for speech. Ollama on the same PC is one answer route. With a cloud provider selected, the question and relevant text go to that service when an answer is requested.

The current package is for Windows 11 x64. Résumé grounding, adaptive mock sessions, and precomputed packs aren't in the preview. Our speech comparison uses a synthetic corpus on one PC. Results can differ for real calls and devices.

To try it, install the verified Windows package and connect a model you can use, or set up a speech model and choose transcription only. I'd be interested in an install step that needs clarification or a short synthetic question where detection misses a correction.

## Facts to use in your own words

- Start with the reason you personally wanted the tool and the one workflow you can demonstrate.
- Explain one trade-off, such as local speech plus an optional remote answer provider, or question revisions arriving while an answer streams.
- Name the Windows preview scope, what is still experimental, and what the benchmark does not measure.
- Point to a working download and ask for a narrow reproduction. Reply yourself and keep the discussion about the project and trade-offs.
