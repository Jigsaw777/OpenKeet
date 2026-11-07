# Native desktop UI validation

Baseline B [overview](../../benchmarks/output/baseline-3.0/ui/first-run.png) used a large welcome panel. The [new dark overview](../../target/ui-smoke/first-run.png) puts sample start, audio setup, and capture status in the initial viewport. [Light overview](../../target/ui-smoke/overview-light.png) uses a light canvas and light navigation rail.

| Screen | Checked behavior | Screenshot |
| --- | --- | --- |
| Overview | Keyboard skip link, primary session action, dark/light/system switching, visible status | [Dark overview](../../target/ui-smoke/first-run.png) |
| Live workspace | Checked follow-up, copy, manual and full transcript actions, separate reading regions. Rust tests cover superseded state. | [Split](../../target/ui-smoke/live-long-question.png), [answer view](../../target/ui-smoke/answer-view.png) |
| Focus | 380 × 480 CSS px, keyboard scroll, visible Copy, pin/unpin and return to workspace | [Focus](../../target/ui-smoke/focus.png) |
| Audio setup | Endpoint list, permission, verified model path, missing-model recovery, manual-answer default, pause/resume | [Audio setup](../../target/ui-smoke/audio-setup.png) |
| Saved sessions | Save, transcript search, notes reopen, export controls and confirmed deletion | [Saved sessions](../../target/ui-smoke/saved-sessions.png) |
| Providers | Browser sign-in entry points, masked Claude key field, installed model picker and selected route. Live fixture answers render lists and code and preserve raw text for follow-up context. | [Providers](../../target/ui-smoke/providers.png) |

At the observed desktop window size the answer expanded from 510 to 878 CSS px. The native viewport stayed the same and the full-screen element remained unset. The compact answer received 273 of 480 CSS px with the controls visible. Earlier B measured 501 to 863 CSS px and 273 of 480 CSS px in focus.

Native WebView2 checks covered the 1366 × 768, 1920 × 1080, 2560 × 1440 and 3840 × 2160 layout equivalents at 100%, 125%, 150% and 200% text scaling on Audio Setup. The 650 × 780 narrow overview screenshot uses 150% text. The Dark and Light text pairs met a 4.5:1 threshold. Tested field borders and focus tokens met 3:1. System preference switched under media emulation with reduced motion. These checks are scoped to the tested tokens, controls and simulated viewports. Actual multi-monitor positioning and physical Windows DPI validation are open.

Typography and spacing specifications are in [design system](DESIGN_SYSTEM.md). Screenshot PNGs are local test artifacts. Reproduce them with the [native smoke script](../../apps/desktop/tests/native-smoke.mjs).

## Public interface review

| Reference | Public UI observed | Personal Copilot scope |
| --- | --- | --- |
| Natively | The dashboard separates session start and history. Its guide describes provider settings, a compact overlay and optional automatic answers. [Dashboard and demo](https://github.com/Natively-AI-assistant/natively-cluely-ai-assistant/blob/7fb88cf56f1e1317075776228615a1f79e8b3c0a/README.md) | The matte workspace separates providers, live capture and history. A native Focus window leaves most of its viewport to the answer. Context library and visual system-design answers remain open. |
| Cluely | A thin listening toolbar sits above a larger answer card. The card can reveal transcript and offers manual answers and suggested questions. [Live Insights guide](https://docs.cluely.com/feature/liveinsights) | **Get answer** acts on the detected question, and **Answer view** toggles the transcript and widens reading. Contextual action chips remain open. |
| ParakeetAI | The public dashboard preview shows active and past sessions. The live preview shows separate Answer and Chat actions. The public site shows document grounding and opt-in auto answer. [Product and feature preview](https://www.parakeet-ai.com/) | Current and saved sessions are separate. The app supports typed questions and optional automatic answers. Uploaded resume/document context and generated notes remain open. |

This review covers the public previews. Account-only screens, product behavior under load, and comparative transcription accuracy require separate tests.
