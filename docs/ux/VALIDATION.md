# Native desktop UI validation

Baseline B [overview](../../benchmarks/output/baseline-3.0/ui/first-run.png) used a large welcome panel. The [new dark overview](../../target/ui-smoke/first-run.png) puts sample start, audio setup, and capture status in the initial viewport. [Light overview](../../target/ui-smoke/overview-light.png) uses a light canvas and light navigation rail.

| Screen | Checked behavior | Screenshot |
| --- | --- | --- |
| Overview | Keyboard skip link, primary session action, dark/light/system switching, visible status | [Dark overview](../../target/ui-smoke/first-run.png) |
| Live workspace | Checked follow-up, copy, manual and full transcript actions, separate reading regions. Rust tests cover superseded state. | [Split](../../target/ui-smoke/live-long-question.png), [answer view](../../target/ui-smoke/answer-view.png) |
| Focus | 380 × 480 CSS px, keyboard scroll, visible Copy, pin/unpin and return to workspace | [Focus](../../target/ui-smoke/focus.png) |
| Audio setup | Endpoint list, permission, verified model path, missing-model recovery, manual-answer default, pause/resume | [Audio setup](../../target/ui-smoke/audio-setup.png) |
| Saved sessions | Save, transcript search, notes reopen, export controls and confirmed deletion | [Saved sessions](../../target/ui-smoke/saved-sessions.png) |

At the observed desktop window size the answer expanded from 510 to 878 CSS px. The native viewport stayed the same and the full-screen element remained unset. The compact answer received 273 of 480 CSS px with the controls visible. Earlier B measured 501 to 863 CSS px and 273 of 480 CSS px in focus.

Native WebView2 checks covered the 1366 × 768, 1920 × 1080, 2560 × 1440 and 3840 × 2160 layout equivalents at 100%, 125%, 150% and 200% text scaling on Audio Setup. The 650 × 780 narrow overview screenshot uses 150% text. The Dark and Light text pairs met a 4.5:1 threshold. Tested field borders and focus tokens met 3:1. System preference switched under media emulation with reduced motion. These checks are scoped to the tested tokens, controls and simulated viewports. Actual multi-monitor positioning and physical Windows DPI validation are open.

Typography and spacing specifications are in [design system](DESIGN_SYSTEM.md). Screenshot PNGs are local test artifacts. Reproduce them with the [native smoke script](../../apps/desktop/tests/native-smoke.mjs).
