# Legacy OpenKeet architecture audit (2.0)

This audit records the Python baseline and migration plan. The [current source map](legacy-python.md#source-map) lists the Rust replacements.

Baseline checkout `69616f4` on `main`, 6 November 2026. The product is a Python 3.12/Tkinter desktop app. Its launcher, provider choices, profiles, and full-transcript workflow are the compatibility baseline during migration. There was no existing repository graph or tracked test suite at this revision. Earlier tests are visible in `1b30f47` and were removed in `18d0edb`.

## Findings from the checkout

| Priority | Observed | Consequence or open question |
| --- | --- | --- |
| P0 | [`audio.py:85`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/audio.py#L85) loads English models on CPU/int8. [`SpeechSegmenter`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/audio.py#L48) uses fixed RMS `0.012`, eight voiced blocks, 300 ms pre-roll, and a roughly 24-second cap. | Any speech whose peak is below the threshold is mathematically excluded. How often that occurs in the owner's setup requires consented or licensed representative audio. |
| P0 | [`audio.py:133–142`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/audio.py#L133) bounds capture at 400 × 30 ms. [`live.py:23–49`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/live.py#L23) keeps eight finals and one replaceable partial, evicting the oldest final on overflow. | Both paths report loss, which can still remove a constraint from the question. Measure gaps, decode load, and missed utterances independently of WER. |
| P1 | [`audio.py:230–256`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/audio.py#L230) decodes cumulative partials around every 900 ms with greedy decoding. Final uses beam 3 and a VAD filter. | Provisional and final text can diverge, with repeated work under load. Historical validation observes one Kotlin error. Its cause is unresolved. |
| P1 | [`audio.py:146–182`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/audio.py#L146) uses separate system and input APIs. Capture stamps are assigned on enqueue. A default output switch or unplug ends capture. | Source and device identity need to survive all pipeline stages. Output may contain multiple speakers mixed together. |
| P1 | [`live.py:235–247`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/live.py#L235) groups by prefixes, gaps, and prior state. [`live.py:342–381`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/live.py#L342) expects a JSON line before any answer text. [`app.py:795`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/app.py#L795) combines orchestration and rendering on the Tk event pump. | Fixture coverage is needed for delayed finals, changed boundaries, and immediate next questions. Typed revisions and checked-result gating can remove UI/provider races. |
| P1 | [`providers.py:104–134`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/providers.py#L104) invokes subscription-backed CLI processes. [`providers.py:214–222`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/providers.py#L214) launches Claude login. [`start_windows.ps1`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/start_windows.ps1) can install separate tools. | The archived revision keeps those adapters. The desktop uses separately documented provider routes and a Windows installer. |
| P2 | [`core.py:76–133`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/core.py#L76) reads local profiles and text/DOCX/PDF files. [`core.py:227`](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/core.py#L227) uses a fixed Ollama loopback URL. | Preserve profile import and fail closed on invalid formats. Reassess decompression limits, prompt-data separation, and redirects before reusing imports/providers. |

[`VALIDATION.md`](../VALIDATION.md#november-5-update) reports 1/68 word errors for `small.en` and 7/68 for `base.en` and `tiny.en` on five synthetic questions. Windows launch and one Realtek route had controlled checks. The exact audio and runnable old benchmark were unavailable in the checkout. Production accent, device diversity, long sessions, and meeting codec effects remain open.

## File-by-file migration map

| Current file | Destination and preserved behavior |
| --- | --- |
| `app.py` | React desktop views and Tauri bridge. Preserve manual questions, full transcript as question, stable reading position, profile selection, and clear capture/generation state. |
| `audio.py` | Rust audio adapter and local STT worker. Explicit route selection, timestamps, loss notices, endpoint events, and separated capture/decode. Retain this file for the comparator. |
| `live.py` | Rust reducer and scheduler. Preserve pause-aware continuation, draft vs checked answers, bounded work, and replacement of a history pair when a late constraint revises it. Use validated classification separately from answer content. |
| `providers.py` | Approved opt-in adapters with typed terminal results. Keep current subprocesses isolated in Python during migration. |
| `core.py` | Local context/store and Ollama adapter. Read legacy profile via explicit import, preserve supported document types, and restrict network destinations. |
| `capture_privacy.py`, `window_shortcut.py` | Normal OS focus window and shortcut handling. Report sharing behavior based on verified OS capabilities. |
| `free_cluely_bridge.py` | Legacy-only text adapter. Separate from the desktop IPC boundary. |
| `start_windows.ps1`, `start_windows.vbs`, `start_mac_linux.command`, `START_HERE.md`, `WINDOWS_QUICKSTART.md` | Keep legacy entry points until parity. New Windows build uses an offline-capable desktop binary and a Tauri installer, with an explicit model setup step. |
| `requirements.txt`, `.gitignore` | Keep Python benchmark dependencies. Keep virtual environments, model weights, audio, profiles, credentials, and build outputs out of source. |
| `README.md`, `LIVE_QUESTION_LOGIC.md`, `VALIDATION.md`, `LICENSE` | Preserve legacy docs and MIT notices. Link new milestones, tests, and eventual install instructions separately. |

## Gates and estimates

Estimates are engineering effort for one engineer and assume working tooling. Credential enrollment, participant consent, hardware acquisition, and external reviews have separate lead times.

| Milestone | Gate | Estimate |
| --- | --- | --- |
| M0 Audit and comparator | Reproducible same-input legacy results, prioritized evidence, screen flows, dependency/license inventory, and redacted history scan. | 1–2 days |
| M1 Rust core | CLI replay, versioned event contract, question revision and bounded queue tests, stale/failed answers rejected, full transcript supported. | 2–4 days |
| M2 Windows audio | Selected WASAPI output and mic capture, recovery with visible loss, VAD and native local ASR. Same-corpus comparison on real playback and representative audio. | 1–2 weeks |
| M3 Desktop UI | Packaged practice and live flows, focus window, context and settings, visual inspection at 100% and 150%, and keyboard/scroll coverage. | 1–2 weeks |
| M4 Providers | Local Ollama and approved opt-in ChatGPT plan/Claude API with secure credential storage, cancellation, errors, and real consent/denial verification. | 1–2 weeks after access |
| M5 Release candidate | Windows installer smoke test, migration and export/delete checks, licenses, SBOM, security review, and private release package. Owner controls publication and any payment activation. | 1–2 weeks |

## Evaluation matrix

Windows 11 x64 build 26200, i5-1335U (10 cores / 12 threads), 16 GiB installed RAM, Intel Iris Xe. Legacy measurements use Python 3.12.10, faster-whisper 1.2.1 and CTranslate2 4.8.5 on CPU/int8. Compare on identical WAV hashes and text normalization. Measure GPU acceleration separately when a supported runtime and hardware are verified.

| Backend | Initial coverage | Later gate |
| --- | --- | --- |
| faster-whisper `tiny.en`, `base.en`, `small.en` | Cached CPU models, paced input replay through the production listener. | Known playback on Realtek and headset, gain variation, and meeting codec. [faster-whisper](https://github.com/SYSTRAN/faster-whisper) |
| whisper.cpp with WebRTC VAD | CPU `small.en` for same-size comparison. Measure the end-to-end Rust pipeline. | Compare quantized `large-v3-turbo`, `medium.en`, and distilled variants subject to memory/RTF and license. [whisper.cpp](https://github.com/ggml-org/whisper.cpp), [Distil-Whisper](https://github.com/huggingface/distil-whisper) |
| Parakeet-TDT v3 | Candidate for a native/ONNX benchmark. Model card lists CC BY 4.0. | Confirm runtime and attribution obligations, then test throughput, English accents, and technical words. [NVIDIA model card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) |
| Deepgram / Speechmatics | Design comparison only. | Obtain explicit cloud consent, budget, and scoped credentials, then compare interim stability, endpointing, rate limits, and cost. [Deepgram endpointing](https://developers.deepgram.com/docs/understand-endpointing-interim-results/), [Speechmatics Realtime](https://docs.speechmatics.com/speech-to-text/realtime/quickstart) |

The initial pack covers 32 locally synthesized clips with three installed US/UK English voices. Cases include below-threshold speech, white noise, technical terms and cash/cache, a short follow-up, late constraints, silence, and a non-speech tone. The next corpus gate calls for roughly 100 clips across 10 voices, including Indian English, licensed or explicitly consented recordings, codecs, and device changes. Report WER, CER, term recall, missed speech, false transcripts, partial and final latency, process load, RTF, and audio loss. A 25% relative WER reduction on representative data is a target. Latency and failure rates must stay usable at the chosen accuracy preset.

## Provider and billing boundaries

- Eligible local/open-source ChatGPT plan use requires an issued client and separate plan permission. Use system-browser OIDC/PKCE, an installation host ID, account-scoped models and the supported Responses stream, including terminal errors. A private project must verify enrollment eligibility. Paid or remotely hosted uses require a separate access path. [Quickstart](https://developers.openai.com/siwc/quickstart), [Plan usage](https://developers.openai.com/siwc/token-sharing-open-source), [Inference contract](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference), [Preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations), [App server option](https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server)
- Claude API BYOK uses a Console key and bills the user's API account. Treat any separate commercial distribution program for Claude Code under its own terms. The desktop product will use the ordinary public API path after opt-in. [Anthropic legal and compliance](https://code.claude.com/docs/en/legal-and-compliance), [Claude API](https://platform.claude.com/docs/en/api/overview)
- Local practice needs no product account. Future product billing covers an independently approved managed service. GitHub Sponsors can support the free project after eligibility and payout checks. Keep MIT notices. [GitHub Sponsors](https://docs.github.com/en/sponsors/getting-started-with-github-sponsors/about-github-sponsors), [Repository licensing](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository)
