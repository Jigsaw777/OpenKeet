# Live questions, predictions and pauses

This describes the archived Python prototype. The current desktop uses the [Rust session protocol](docs/adr/0002-session-protocol.md).

The important distinction is between a **capture segment**, a **candidate logical question**, and a **checked answer**. A short pause can close a capture segment while the person is still building the same question.

## Implemented flow

1. Capture system audio continuously and detect speech locally.
2. Offer a cumulative partial transcript about every 0.9 seconds while speech continues. If recognition falls behind, replace an old queued partial snapshot rather than processing every snapshot late.
3. Once there is enough wording, request an early answer marked **PROVISIONAL DRAFT**. The model is instructed to answer only the words already heard and to wait if no question is clear. This is a hypothesis, not a commitment to an unheard ending.
4. Close a capture segment after roughly 750 ms of silence, then buffer it as part of the current candidate question.
5. Wait for the configurable **Question pause**, initially **2.0 seconds** since the last captured speech. A clause ending with “but,” “if,” “using,” or “without” waits at least **2.2 seconds**. If speech resumes before the new transcript arrives, the pending checked-answer request is held back.
6. Fast continuity rules join short gaps and phrases such as “but,” “and,” “what if,” “using,” “actually” and “I mean.” Explicit “next/new/different question” markers start another candidate. The selected model then examines the chronological fragments and previous completed question to decide **new**, **continue** or **incomplete**.
7. The model's decision and consolidated question precede its answer in a machine-readable header. The app validates and hides that header before displaying checked answer text. If the decision is **incomplete**, the app keeps listening and does not commit a guessed answer.
8. New constraints mark an older answer outdated. Once the revised question is ready, a stale in-flight prediction is stopped and replaced. A continuation revises the prior question/answer pair in history. A distinct question creates a new pair. Drafts, obsolete versions and incomplete questions are excluded from history.

The first early attempt can legitimately say “Listening for the question…” during a long introduction. One additional early attempt is allowed after at least four more words; it is not retried on every partial transcript. Automatic checked revisions can still occur when the speaker adds further constraints after an earlier apparent question ending.

## Example

| Heard speech | Interpretation / behavior |
| --- | --- |
| “How would you protect shared state…” | Start a provisional draft once a recognizable question is present. |
| A brief pause | Keep buffering; do not treat silence alone as a new question. |
| “…but without blocking a coroutine's underlying thread?” | Extend the same candidate and mark the earlier wording outdated. |
| Debounced pause | Ask the model to validate the consolidated question and revise for the non-blocking constraint. |
| “Next question: explain database indexes.” | Start a distinct candidate and a new answer entry. |

The example illustrates the scheduling and revision rule. It is not a claim that any model will always select the correct Kotlin primitive or classify every boundary correctly.

## Semantic decision protocol

For an automatic checked-answer request, the selected model must start with one JSON line:

```json
{"relation":"continue","question":"How would you protect shared state without blocking the coroutine's underlying thread?"}
```

The visible answer follows on subsequent lines. A malformed or unfinished header is reported as an error instead of silently treating the answer as validated. The semantic check is part of the same model request as the checked answer; it is not an extra separate classification request. Early drafts are separate requests and use subscription allowance too.

## Windows screen sharing

Windows capture exclusion is checked before showing the main window, dropdowns, and file-path prompts. Verify the receiving view in the exact meeting application and sharing mode.

## What the tests establish

See [validation](VALIDATION.md) for automated and native Windows checks. If transcription falls behind and evicts a completed segment, the app discards that segment's partial text, clears the obsolete draft, and continues with the available completed speech.

There is no guarantee that an answer will arrive before every question finishes. Short questions, slow transcription, CLI startup and network/model time can make the first draft arrive later. The app reports actual first-text timings rather than promising instant output.

## Relationship to the GitHub reference

The reference was inspected at commit `712ddb3474af22886cd47786f7bb0f0a5bf526df`, including:

- [`electron/LLMHelper.ts`](https://github.com/Prat011/free-cluely/blob/712ddb3474af22886cd47786f7bb0f0a5bf526df/electron/LLMHelper.ts): Gemini/Ollama providers and the text generation interface.
- [`electron/WindowHelper.ts`](https://github.com/Prat011/free-cluely/blob/712ddb3474af22886cd47786f7bb0f0a5bf526df/electron/WindowHelper.ts): overlay window setup and Electron content protection.
- [`src/_pages/Solutions.tsx`](https://github.com/Prat011/free-cluely/blob/712ddb3474af22886cd47786f7bb0f0a5bf526df/src/_pages/Solutions.tsx): the microphone recording path, including its five-second recording interval.

This package is a separate Python implementation inspired by that source review, plus a text bridge for the reference repository. It is not a fork or a claim that the upstream Electron app was modified, built or tested here.
