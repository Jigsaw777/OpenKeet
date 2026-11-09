# OpenKeet brand guide

OpenKeet is a personal AI interview assistant for Windows. It helps candidates understand a question, work out a stronger answer and approach the conversation with composure. This guide covers strategy, identity, voice and reusable copy. Tokens, components and screens are in the [design system](DESIGN_SYSTEM.md).

## A. Strategy

**Purpose.** When an important question comes your way, you can approach it with clarity and confidence.

**Positioning.** OpenKeet is a Windows interview copilot for preparation, practice, and real-time answer suggestions in sessions that allow AI help.

**Category.** Personal AI interview assistant.

**Audience.**

| Group | What they need from OpenKeet |
| --- | --- |
| Software engineers before technical rounds | Accurate technical terms, code-friendly answers, low latency |
| Candidates practising behavioural and HR rounds | Calm structure, plain language, no judgement |
| Experienced professionals changing roles | Credible, concise framing of existing experience |
| Students and early-career applicants | Reassurance without hype, clear next steps |
| Architects preparing system-design discussions | Room for longer, structured explanations |

**Personality.** Primary archetype: the Mentor. Secondary: the Sage. OpenKeet behaves like a knowledgeable interview coach who explains clearly and helps you improve. It is never a robotic answer machine, a scoreboard or an intimidating examiner.

**Emotional promise.** Calm under pressure. Confidence without arrogance. Intelligence without clutter. Support without dependency.

**Tagline hierarchy.**

1. Primary, everywhere: **Interview smarter. Answer better.**
2. Supporting lines for sections and campaigns: *Be ready for the questions that matter.* · *Prepare with purpose. Answer with confidence.* · *Your interview, a little more confident.* · *Practice better. Think clearer. Respond stronger.*

**Messaging pillars.**

| Pillar | Proof in the current product |
| --- | --- |
| Understand the question | Live local transcription, question detection, follow-up linking, revisions as the question settles |
| Answer with structure | Suggested answers stream into a readable panel with lists and code blocks |
| Runs on your machine | Speech models run on the PC (Whisper `small.en` or optional NVIDIA Parakeet v3). Ollama answers can stay local too |
| You stay in control | Explicit Start, Pause and Stop, visible listening state, consent prompt, local history you can export or delete |
| Bring your own model | ChatGPT through your account, Claude through your Console key, or local Ollama |

**Differentiation.** Real-time question understanding, local speech recognition, transparent model choice and local history, in a calm desktop experience designed for interviews rather than meetings.

**Honest use.** Describe live transcription and answer suggestions plainly. Ask users to follow interview and screen-sharing rules and get permission before capturing audio. Describe screen-share privacy as an optional Windows setting for OpenKeet's own windows. Its effect depends on the capture method. [Live assistance guide](../live-assistance.md)

## B. Identity

### Logo

The approved direction is concept three. A Midnight Navy rounded-square tile holds a teal question bubble with a white question mark, a white connecting slash and a lime answer bubble with two text lines. A small lime spark sits top right. Read as one symbol: a question becoming a useful answer.

| Asset | File | Use |
| --- | --- | --- |
| Horizontal logo, dark background | `assets/openkeet-logo-on-dark.svg`, `.png` | App header, README in dark mode, release notes |
| Horizontal logo, light background | `assets/openkeet-logo-on-light.svg`, `.png` | Documents, README in light mode |
| App icon, no wordmark | `assets/openkeet-mark.svg`, `openkeet-mark-1024.png` | Store listings, splash, about |
| Outlined app icon | `assets/openkeet-mark-outlined.svg` | Navy tile on navy surfaces |
| Compact monogram, 16–24 px | `assets/openkeet-mark-small.svg` | Tray and tiny UI |
| Monochrome light / dark | `assets/openkeet-logo-mono-*.svg`, `openkeet-mark-mono-*.svg` | One-colour print, watermarks, empty states |
| Windows app and installer icon | `apps/desktop/src-tauri/icons/icon.ico` | 16, 20 and 24 px use the compact mark; 32–256 px use the full mark |
| Tray icon | `apps/desktop/src-tauri/icons/tray.png` | 32 px compact mark, scaled by Windows |
| GitHub and social avatar | `assets/openkeet-avatar.svg`, `openkeet-avatar-400.png` | Full-bleed navy, mark inside the centre circle for round crops |

The vector mark traces the supplied concept-three artwork on a 64-unit grid. It replaces the earlier upscaled raster icon, which had soft edges and white corners baked in. All files are vector-first with transparent backgrounds outside the tile. The wordmark is outlined from Manrope ExtraBold (SIL Open Font License), so it renders the same without the font installed.

**Wordmark.** Always **OpenKeet**: capital O, capital K, one word. On dark, "Open" is Soft White `#F5F7FB` and "Keet" is Electric Teal `#10C7C2`. On light, "Open" is Midnight Navy `#0B1020` and "Keet" is Deep Teal `#0A7C79` (5.0:1 on white).

**Clear space.** Keep space equal to the tile's corner radius (14/64 of the tile height) on every side. In the horizontal logo the gap between tile and wordmark is 9/64 of the tile height.

**Minimum size.** Full mark 32 px. Below that use the compact mark. Horizontal logo 24 px tall on screen, 10 mm in print.

**Small sizes.** The compact mark redraws the symbol on 45° pixel diagonals. It drops the tails, question mark, spark and text lines, which blur below 32 px.

**Navy on navy.** When the tile sits on a Midnight Navy surface, use the outlined variant. Its `#2A3552` hairline separates the tile from the background.

**Incorrect use.** Do not stretch, rotate, recolour the shapes, swap teal and lime, add gradients, glows or drop shadows, or put the dark-text wordmark on navy. Do not respell the name (OpenKit, Openkeet, OPENKEET). Do not replace the mark with stars, brains, robots or microphones, or crowd the clear space.

### Colour

| Token | Hex | Role |
| --- | --- | --- |
| Midnight Navy | `#0B1020` | Dominant dark surface, light-theme text |
| Electric Teal | `#10C7C2` | Primary interaction and focus |
| Signal Lime | `#A7F432` | Insight, readiness, completed answers, used sparingly |
| Soft White | `#F5F7FB` | Reading text on dark, light canvas |
| Slate Navy | `#151C2F` | Cards and secondary surfaces |
| Cool Gray | `#98A2B3` | Reference muted tone. Muted UI text uses `#A8B1C2` for contrast |
| Mist Gray | `#E5EAF2` | Light dividers and raised light surfaces |
| Success Mint | `#D9FF8A` | Success text on dark |
| Warning Amber | `#F5B942` | Warnings |
| Error Coral | `#F05D6C` | Brand coral. Text uses `#FF8A95` on dark and `#B3243A` on light for contrast |

Teal is for things you can act on and live listening. Lime appears only when something is ready or done. Never put both on every component. The functional shades and semantic tokens are defined in the [design system](DESIGN_SYSTEM.md#c-theme-system).

### Typography

| Use | Face | Why |
| --- | --- | --- |
| Desktop UI | Segoe UI Variable Text and Display, falling back to Segoe UI | Native Windows rendering, DPI-aware, no bundled font |
| Code, model IDs, shortcuts | Cascadia Mono, falling back to Consolas | Ships with Windows 11 Terminal; Consolas everywhere |
| Logotype only | Manrope ExtraBold, outlined (OFL) | Matches the wordmark geometry. Not a UI font |

The app bundles no fonts. Segoe UI Variable gives a native feel and avoids roughly 500 KB of font files.

### Iconography

One family: 24 px grid, 1.6 px rounded strokes, no fills, `currentColor`. The app's inline set (`Icon` in `components.tsx`) follows these rules. Lucide icons match them if more are needed. Do not mix libraries. Sizes: 16 px in dense rows, 18 px in buttons and navigation, 24 px for empty states. Icons inherit text colour, use the muted tone when disabled, and always sit beside a text label or carry an accessible name. The brand mark is identity only, never a functional icon. Suggested mappings if more icons are needed: live session `radio`, transcription `audio-lines`, technical `code`, system design `network`, behavioural `users`, local processing `cpu`, API provider `plug`, knowledge `book-open`, history `clock`, privacy `lock`, warning `triangle-alert`, error `circle-alert`.

### Imagery

Prefer real application screenshots, cropped to the live workflow, on Midnight Navy. Diagrams use the token palette with 1.5 px navy-gray strokes. Avoid robots, glowing brains, stock "AI" art and staged handshake photography.

## F. Voice and copy

OpenKeet sounds like a thoughtful interview coach who helps you find clarity when questions get difficult.

**Voice.** Natural, calm, direct, encouraging, intelligent, respectful, concise. Not robotic, hyped, corporate, alarmist or patronising.

**Rules.**

1. Write how a person speaks. Plain English, verbs over nouns.
2. Sentence case for every label, button and heading.
3. Buttons say what happens: "Start listening", "Get answer", "Copy answer".
4. Errors say what happened in user terms, then what to try. No codes unless support needs them.
5. Never promise accuracy. Suggested answers are suggestions.
6. Never imply OpenKeet replaces real experience, preparation or honesty.
7. Never refer to colour alone ("the red message"). Name the place or the label.
8. Use "we" for OpenKeet only in apologies and errors ("We couldn't…").

**Terminology.**

| Use | Avoid |
| --- | --- |
| Suggested answer | AI response, output, generation |
| Question taking shape | Low-confidence prediction |
| Listening, Paused, Session ended | Capture active, idle state |
| Speech model | ASR engine, transcription backend |
| Answer provider | LLM backend |
| Live preview | Interim hypothesis |
| Practice question | Test prompt |

### Microcopy library

*Shipped* marks entries whose heading and supporting text are in the app today. Other entries are ready for screens and states that do not exist yet.

| Moment | Heading | Supporting text | Primary action | Secondary |
| --- | --- | --- | --- | --- |
| First launch | Finish setup | Pick what to listen to, get a speech model, and optionally connect answers. | Start listening | — |
| Ready *(shipped)* | Ready to listen | Answers show in a see-through window on top of your call. | Start listening | — |
| Welcome | Welcome to OpenKeet | Interview smarter. Answer better. Let's get your audio and answers ready. | Get started | — |
| Microphone permission | Allow microphone access | OpenKeet listens only while a session is running. You can pause or stop at any time. | Open Windows settings | Use system audio instead |
| System audio | Choose what to listen to | Pick the speakers or headphones your call plays through. | Refresh | — |
| Speech model *(shipped)* | Speech model | Runs on this PC. | Download | Choose file |
| Local model setup | Run answers on this PC | Install Ollama, pull a model, then refresh. Your questions stay on this machine. | Refresh models | Use a cloud provider |
| API provider | Connect your Claude key | Paste a key from Claude Console. It's stored encrypted for your Windows account. | Save key | Open Claude Console |
| Missing key | Connect answers to get replies | You'll still get a live transcript without them. | Connect answers | Not now |
| Session start *(shipped)* | Starting… | Loading the speech model. This takes a few seconds. | — | Stop |
| Listening *(shipped)* | Listening | Listening for the next question. | Pause | Stop |
| Speech *(shipped)* | Hearing speech | Turning speech into text. | — | — |
| Question emerging | Question taking shape | The question can still change while they speak. | Get answer | — |
| Question heard *(shipped)* | Question heard | Press Get answer when they finish. | Get answer | Answer what they said |
| Answer generating *(shipped)* | Writing answer… | Putting together a suggested answer. | Cancel answer | — |
| Answer ready *(shipped)* | Answer ready | Still listening for the next question. | Copy answer | — |
| Answer failed *(shipped)* | Answer failed | The message above says what to try next. | Retry answer | Choose provider |
| No sound *(shipped)* | Listening | No sound from the selected device yet. Check that the call plays through it. | — | — |
| Device lost *(shipped)* | Audio stopped | The audio device stopped. Press Retry same source. | Retry same source | Stop |
| Microphone failure | We couldn't connect to your microphone | Check your audio settings and try again. | Try again | Choose another device |
| Model unavailable | Your answer provider isn't responding | Check your connection or pick another provider. | Retry | Open answer settings |
| Offline | You're offline | Live transcription keeps working. Cloud answers return when you're back online. | — | Use Ollama |
| Empty transcript *(shipped)* | Nothing said yet | Their words appear here as you listen. Live preview lines can still change. | — | — |
| Empty history *(shipped)* | No saved sessions yet | Your saved practice sessions will appear here. Save one after it ends. | — | — |
| Saving | Saving… | — | — | — |
| Saved *(shipped)* | Notes saved. | — | — | — |
| Copied *(shipped)* | Answer copied | — | — | — |
| Clipboard failure *(shipped)* | We couldn't copy that | Select the answer text and copy it manually. | — | — |
| Update needed *(shipped)* | OpenKeet needs a restart to finish updating | Quit it from the tray and open it again. | — | — |
| Privacy | Local transcription, your choice of answer provider | Audio is transcribed on this PC. Saved sessions stay in your local library. If you choose a cloud answer provider, each answer request sends your question and related text to that service. | Open history | Delete a session |
| Help | Something not working? | Check the audio device, the speech model and the answer provider in that order. | Open settings | Read the guide |

### Marketing copy

**One line.** OpenKeet is your personal AI interview assistant for Windows. Practice questions and shape stronger answers in your own words.

**50 words.** OpenKeet listens to your interview call on Windows, transcribes it locally, and spots the questions as they're asked. Connect ChatGPT, Claude or a local Ollama model to get a suggested answer in a see-through window over your call. Save sessions to review later.

**150 words.** Interviews are hard enough without losing the thread of a question. OpenKeet is a personal AI interview assistant for Windows. It transcribes audio on your PC using Whisper or experimental NVIDIA Parakeet v3. As someone speaks, OpenKeet tracks the question as it takes shape. Get a suggested answer through the provider you choose. Use local Ollama, a Claude Console API key, or the preview sign-in for eligible ChatGPT accounts. Answers appear in a compact see-through window over your call, with readable lists and code blocks for technical rounds. Save a finished session to your local library to search, add notes, export or delete it. OpenKeet helps you think. Your experience is still the answer. Interview smarter. Answer better.

**Launch announcement.** OpenKeet is here. It is a Windows interview copilot that hears the question, keeps it in focus and suggests a clear answer while you stay in control. Speech recognition runs on your PC. Answers come from the provider you pick, including fully local Ollama.

**GitHub introduction.** OpenKeet is a Windows desktop interview copilot built with Tauri 2 and Rust. It captures system audio or a microphone, transcribes locally with whisper.cpp or sherpa-onnx Parakeet, detects questions, and streams suggested answers from ChatGPT, Claude or Ollama into an overlay window. MIT licensed.

**Product Hunt.** Interview smarter. Answer better. OpenKeet listens to your interview on Windows, transcribes locally and suggests structured answers in a see-through window. Bring ChatGPT, Claude or local Ollama.

**LinkedIn.** Interviews reward clear thinking under pressure. I've been building OpenKeet, a Windows copilot that transcribes the conversation on your own PC, spots each question and suggests a structured answer you can adapt. It's built for practice and preparation, and it never replaces your own experience.

**Developer communities.** OpenKeet is a Tauri 2 + Rust interview copilot for Windows. It does WASAPI loopback, WebRTC VAD, local whisper.cpp or Parakeet via sherpa-onnx, question detection in a Rust core, and streaming answers from ChatGPT, Claude or Ollama. MIT licensed. Benchmarks and validation notes are in the repo.

Do not publish testimonials, user counts, accuracy percentages or comparisons with other products unless they are measured and cited.
