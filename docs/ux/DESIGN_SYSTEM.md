# Desktop design system

The workspace uses matte graphite for dark mode and a cool paper surface for light mode. The default is dark. Appearance can follow the system or use either theme explicitly. Native window chrome follows the selected preference.

| Token | Specification at 100% |
| --- | --- |
| Spacing | 4, 8, 12, 16, 24 and 32 CSS px. Use 16 px between panels and 24 px inside dashboard cards. |
| Type | Segoe UI and system fallback. Body 15 px, labels 12–13 px, headings 18 and 28 px, compact answer 15.5 px. |
| Surface | Flat backgrounds with 1 px separators and 12 px card radius. Dark canvas `#111214`, surface `#1a1b1f` and navigation `#0b0c0e`. |
| Controls | Standard buttons 36 px tall, 8 px radius, 3 px keyboard focus outline. Compact controls use less vertical space. |
| Feedback | Explicit text for idle, loading, listening, pause, finishing and recovery. Color supports the text. No animated waveforms. |

## Workspace

Overview groups session start, current status, and the latest checked question. Audio Setup groups source, model and vocabulary, permission and test, and preferences. The live screen keeps recording controls in its heading, followed by a short route and layout bar. Transcript and answer use independent reading regions and retain scroll position through interim updates. Answer view gives the response the full workspace width. The focus window keeps its status and controls above a separately scrollable response. The complete question is one keyboard-accessible expansion away.

The navigation rail becomes horizontal at narrow widths. Capture and appearance controls wrap and retain their labels. App text scaling is independent of Windows display scaling.

Baseline screenshots are in `benchmarks/output/baseline-3.0/ui`. The native WebView2 smoke writes comparison screenshots to `target/ui-smoke`. It tests keyboard skip and reading, light/dark/system, narrow viewports and 100%–200% app text scaling. Emulated viewport sizing is separate from testing Windows display scaling on physical monitors.
