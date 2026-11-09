# OpenKeet design system

The desktop app is Tauri 2 with React 19 and one stylesheet, `apps/desktop/src/style.css`. Every colour, face, radius and motion value used by components is a CSS custom property on `:root`. Themes swap values only, never names. Brand rules and copy are in the [brand guide](BRAND.md).

## C. Theme system

Dark is the flagship theme. Light is a full mapping, not an inversion. **Follow system** resolves to one of them and tracks Windows changes live. The choice persists in `localStorage` and is applied before the first React render, so a light-theme start never paints a dark frame. The native window theme follows through `setTheme`. The overlay window is always dark so text stays readable over any call.

### Colour tokens

| Token | Dark | Light | Role |
| --- | --- | --- | --- |
| `--canvas` | `#0b1020` | `#f5f7fb` | Primary background |
| `--rail` | `#080c18` | `#fff` | Header |
| `--surface` | `#151c2f` | `#fff` | Cards, inputs, default buttons |
| `--soft` | `#1e2740` | `#edf1f7` | Secondary fill: question block, pills, code |
| `--raised` | `#253050` | `#e5eaf2` | Hover |
| `--rail-selected` | `#1a2238` | `#e5eaf2` | Active navigation |
| `--ink` | `#f5f7fb` | `#0b1020` | Primary text |
| `--muted` | `#a8b1c2` | `#4a5468` | Secondary text, eyebrows, metadata |
| `--disabled-ink` | `#7d879c` | `#8a93a5` | Disabled controls |
| `--rule` | `#2a3552` | `#d5dce7` | Borders and dividers |
| `--field-border` | `#6b7793` | `#7a859a` | Input outlines, final transcript rule |
| `--focus` | `#7eebe6` | `#08716e` | Focus ring |
| `--accent` | `#10c7c2` | `#08716e` | Text links, follow-up labels, the question rule |
| `--primary` / `--primary-hover` / `--primary-ink` | `#10c7c2` / `#45d8d4` / `#0b1020` | `#08716e` / `#065c59` / `#fff` | Primary button |
| `--live` / `--live-bg` | teal / teal 12% | deep teal / 10% | Listening state, answer being written |
| `--busy` | teal | deep teal | Processing ring |
| `--ok` | `#a7f432` | `#3d7000` | Done indicator |
| `--insight` / `--insight-ink` | `#a7f432` / `#0b1020` | same | Lime fills with navy glyphs: setup checks, answer rule |
| `--insight-text` / `--insight-bg` | `#d9ff8a` / lime 12% | `#3d7000` / olive 10% | Answer ready and copied labels |
| `--warning` / `--warning-bg` | `#f5b942` / `#2a2414` | `#7a4f00` / `#fff4dc` | Notices that need attention |
| `--danger` / `--danger-bg` | `#ff8a95` / `#2e1822` | `#b3243a` / `#fdecee` | Destructive actions: Stop, Delete, Clear |
| `--wordmark-open` / `--wordmark-keet` | `#f5f7fb` / `#10c7c2` | `#0b1020` / `#0a7c79` | Header logo |
| `--mark-edge` | `#2a3552` | transparent | Hairline around the navy tile |
| `--overlay-rgb` | `11, 16, 32` | n/a | Overlay background, alpha set by the opacity slider |

**Interview semantics** map onto these tokens: listening `--live`, processing `--busy`, question detected `--accent`, provisional question `--accent` dashed, answer ready `--insight`, disconnected and recovery `--warning`, destructive `--danger`. A local model or a cloud provider is named in text (for example "Ollama · qwen3:4b"). It does not get its own colour.

**High contrast.** With Windows *more contrast* (`prefers-contrast: more`), muted text becomes full ink and dividers use the stronger field border. In Windows High Contrast themes (`forced-colors: active`), system colours take over. Status shapes get a `CanvasText` outline so they stay visible, and the focus ring uses `Highlight`.

### Contrast

Computed with the WCAG 2.x relative-luminance formula on the shipped values. The native smoke test re-checks seven of these pairs in both themes on every run.

| Pair | Dark | Light | Target |
| --- | ---: | ---: | ---: |
| Body text on surface | 15.8:1 | 18.93:1 | 4.5 |
| Muted text on surface | 7.86:1 | 7.61:1 | 4.5 |
| Muted text on secondary fill | 6.86:1 | 6.71:1 | 4.5 |
| Header muted text | 9.04:1 | 7.61:1 | 4.5 |
| Accent text on secondary fill | 7.02:1 | 5.15:1 | 4.5 |
| Primary button text | 8.99:1 | 5.84:1 | 4.5 |
| Field border on surface | 3.78:1 | 3.72:1 | 3 |
| Focus ring on canvas | 13.44:1 | 5.44:1 | 3 |
| Warning text on its fill | 8.74:1 | 6.53:1 | 4.5 |
| Danger text on surface | 7.52:1 | 6.50:1 | 4.5 |
| Done indicator on secondary fill | 11.01:1 | 5.27:1 | 3 |
| Disabled text on its fill | 4.10:1 | 2.73:1 | exempt |

Lime is never used as text on white. On light surfaces it appears only as a fill behind navy glyphs (14.1:1).

### Type

Base size is 15 px at 100%. The Text size setting rescales the root to 125–200%, and every size below is in `rem` so the layout follows. Windows display scaling applies on top.

| Role | Face | Size / line height | Weight | Tracking | Use |
| --- | --- | --- | --- | --- | --- |
| Display | Segoe UI Variable Display | 1.87rem / 1.16 | 700 | −0.02em | Page titles: Finish setup, Settings |
| Section heading | Display | 1.22rem / 1.25 | 650 | −0.01em | Card titles |
| Card heading | Display | 1.02rem / 1.3 | 650 | 0 | Answer headings (`h3`) |
| Body | Segoe UI Variable Text | 1rem / 1.52 | 400 | 0 | Answers, transcript, forms |
| Compact body | Text | 0.86rem / 1.45 | 400 | 0 | Hints, metadata |
| Label | Text | 0.89rem | 650 | 0 | Field labels |
| Button | Text | 1rem | 600 | 0 | All buttons |
| Caption | Text | 0.75rem | 650 | 0 | Pills, timestamps |
| Eyebrow | Text | 0.69rem | 700 | 0.12em, uppercase | Section context: What was said |
| Code | Cascadia Mono | 0.91em | 400 | 0 | Inline and block code |
| Shortcut | Cascadia Mono | 0.78em | 600 | 0 | `kbd` keycaps |

Answers keep `pre-wrap` paragraphs, a 0.6rem paragraph gap and list items spaced 0.48em. The answer view caps the reading width at the window, and split view keeps the transcript beside it.

### Space, shape, depth, motion

- **Spacing** uses a 4 px grid at 100%: `--space-1` 4 px, `-2` 8, `-3` 12, `-4` 16, `-6` 24, `-8` 32.
- **Radius**: `--radius-sm` 6 px for code, `--radius-md` 8 px for buttons and inputs, `--radius` 12 px for cards, 999 px for pills. The brand tile uses 22% of its size.
- **Borders**: 1 px rules. State rules are 2–3 px: transcript 2, question 3, answer 3.
- **Elevation**: borders, not shadows. Overlay text gets a 2 px shadow for legibility over video.
- **Controls**: 2.4rem minimum target (36 px at 100%), 1.95rem in the compact overlay. The window minimum is 600 × 530, and the overlay minimum is 340 × 420.
- **Motion**: 120 ms colour transitions on `cubic-bezier(.2, .7, .2, 1)`, a 1 px press offset, and a 2 s opacity pulse on listening and processing indicators. No scaling, bouncing or glow. `prefers-reduced-motion` removes transitions, the press offset and the pulse.
- **Layers**: skip link 10, sticky header 2, content 0. There are no modals or popovers yet. Reserve 20 for menus, 30 for modals and 40 for toasts.

## D. Components

States apply to every interactive component. **Hover** moves to `--raised` or the hover variant. **Focus** draws a 3 px `--focus` outline offset 3 px; mouse clicks never show it, keyboard focus always does. **Active** drops 1 px. **Disabled** uses `--soft` fill with `--disabled-ink`, keeps the border and drops the pointer cursor. Opacity is never used for disabled states.

| Component | Size and spacing | Type | Tokens | Notes |
| --- | --- | --- | --- | --- |
| Primary button | 2.4rem tall, 12 px inline padding, 8 px gap to icon | Button | `--primary`, `--primary-ink`, `--primary-hover` | One per view. Large variant 2.9rem for Start listening |
| Secondary button | Same | Button | `--surface`, `--rule`, `--ink` | Default `button` |
| Tertiary / text button | 1.8rem tall, no padding | Button | `--accent` | Underline on hover. Used for Show transcript, Change |
| Destructive | Secondary shape | Button | `--danger` text | Stop, Delete saved session, Clear current session |
| Icon button | 2.4rem square | — | Secondary tokens | Needs `aria-label` or visible text |
| Navigation item | 2rem tall | 0.87rem / 600 | transparent → `--rail-selected` with `--rule` | `aria-current="page"` marks the active item |
| Settings tabs | Same as navigation | — | Same | Answers, Audio and display |
| Input, select, textarea | 8 px radius, 0.56rem × 0.7rem padding | Body | `--surface`, `--field-border` | Labels sit above, hints below in compact muted text |
| Model selector | Select plus Refresh models | Body | Input tokens | Shows the provider and model ID in mono when available |
| Checkbox | 1.1rem | Body | `accent-color: --accent` | Label text is clickable |
| Slider | 6rem | — | `accent-color: --accent` | Overlay opacity, 30–100% |
| Status pill | 1.65rem tall, 999 px radius | Caption | `--soft`, `--rule`, `--ink` | A shape plus text: dot idle or listening, ring processing, lime dot ready, amber diamond attention |
| Badge / soft label | Same | Caption | `--soft`, `--muted` | Revision 2, source names. Answer status variants use live, insight or warning fills |
| Live status | 0.85rem indicator | 1.15rem / 650 title, muted detail | Status shapes | One sentence of what is happening and what to do |
| Card | 12 px radius, 1 px `--rule`, 16–24 px padding | — | `--surface` | No shadow |
| Checklist step | 1.65rem numbered disc | Label | `--field-border` disc, lime fill with navy check when done | Setup steps |
| Notice | 10 px radius, 8 × 12 px padding | Compact | `--warning` on `--warning-bg` | `role="alert"` for problems |
| Toast | Notice shape | Compact | `--insight-text` on `--insight-bg` | `role="status"`. Answer copied |
| Empty state | Mono mark 24 px, title, one line | Label + compact | `--muted` | Says what will appear and when |
| Transcript row | 2 px left rule, 8 px padding | Body, caption timestamp | `--field-border` final, dashed `--rule` live preview | Labelled Final or Live preview, never colour alone |
| Question block | 3 px left rule, `--soft` fill | Eyebrow + 650 text | `--accent`, dashed while provisional | Eyebrow: Question taking shape, Question, or Current question when empty |
| Answer card | Card with a 3 px inset rule on the body | Body | `--accent` while writing, `--insight` when checked, warning label when superseded | Eyebrow: Suggested answer. Footer actions: Get answer, Cancel answer, Copy answer |
| Code block | 6 px radius, 0.55 × 0.7em padding, horizontal scroll | Code | `--soft` | Focusable, named "Code example" |
| Keyboard shortcut | Keycap with 2 px bottom border | Shortcut | `--soft`, `--rule` | Ctrl+Alt+I |
| Progress / meter | 0.65rem bar | — | `--accent` | Input level |
| Split panel | Two columns 1 : 1.45 | — | — | Collapses to one column under 900 px |
| List row (library) | Full-width button | Label + compact meta | `--accent` border when selected | `aria-pressed` |

## E. Screens

### Built today

**Start (Session, idle).** Order: title (Finish setup or Ready to listen), then three numbered steps (Listen to, Speech model, Answers), then the primary action, then the permission note. Steps tick lime when complete. Start listening stays disabled until a device and a verified model exist. The card is 48rem wide.

**Live session.** Order: live bar (eyebrow, live status, Pause or Resume, Stop, Overlay), session meta (source, provider, transcript toggle), then the split view. *What was said* (transcript) sits left and the question plus *Suggested answer* sits right. Answer view hides the transcript and widens the answer. Ask a question, session history, save and clear follow below.

**Overlay.** A 380 × 480 see-through window for the call itself. Order: live status and Hide, controls and opacity, then the compact question (expandable) and the answer, with Copy answer and Main window always in view. The background is Midnight Navy at 30–100%. Text stays opaque.

**Settings → Answers.** Pick a provider (ChatGPT, Claude, Ollama), sign in or paste a key, choose a model, then confirm the active provider.

**Settings → Audio and display.** Words to recognize, Short follow-up mode, Speech engine, Speech model file or folder, Appearance, Text size, Screen-share privacy on Windows, then Audio diagnostics during a live session.

**History.** Search, a result list, and a selected session with export (Markdown, JSON, TXT), notes and delete.

### Question-state language

| State | Where | Shape | Copy |
| --- | --- | --- | --- |
| Listening | Status | Teal dot, slow pulse | Listening · Listening for the next question. |
| Speech detected | Status | Teal dot | Hearing speech |
| Question emerging | Question block | Dashed teal rule | Question taking shape |
| Question identified | Question block | Solid teal rule | Question |
| Answer generating | Status, answer label | Teal ring, teal answer rule | Writing answer… · Writing |
| Answer ready | Status, answer label | Lime dot, lime answer rule | Answer ready · Complete |
| Answer updated | Answer label | Lime rule, revision badge | Revision 2 |
| Transcription unavailable | Status | Amber diamond | Audio stopped |
| Model unavailable | Notice | Amber notice | Your answer provider isn't responding |

Only the listening and processing indicators animate, with a 2 s opacity pulse. Nothing flashes.

## Implementation notes

- **Tokens**: `style.css` `:root` holds the dark theme, `[data-theme="light"]` overrides values, and the contrast media queries come last. Components reference only semantic tokens. The one literal outside the tokens is the brand tile in `brand.tsx`, which is fixed by the logo rules.
- **Logo**: `apps/desktop/src/brand.tsx` exports `Logo` (tile plus outlined wordmark, coloured by `--wordmark-*`) and `Mark` (mono silhouette for empty states). Its geometry matches `docs/brand/assets`.
- **Icons**: `icons/icon.ico` holds 16–24 px compact and 32–256 px full entries. Tauri uses the largest entry for the window, and `icons/tray.png` is loaded with `include_image!` for the tray. `apps/desktop/icon.svg` is the source mark.
- **Theme start-up**: `resolvedTheme` in `main.tsx` sets `data-theme` at module load, before React renders.
- **Validation**: `npm test`, `npm run test:native` with dark and light contrast checks, and the rendered screenshots in `target/ui-smoke/`.
