# OpenKeet desktop wireframes

Use matte charcoal with near-black navigation, gray borders, and a muted steel accent for active states. Dark is the default. The optional light palette uses neutral grays. Instrument labels and times use tabular numerals. Use reusable controls with visible focus rings, normal desktop windows, and explicit source and locality labels. A distinct timeline presents speech and answer revisions without moving the question box on every partial.

Product references inform separate setup and live work, a compact session view, a persistent prompt, and accessible transcript/answer navigation. The design uses original type, layout, icons, and copy. [Cluely Live Insights](https://docs.cluely.com/feature/liveinsights), [ParakeetAI](https://www.parakeet-ai.com/)

## 1. Dashboard and first run

```text
┌──────────────────────────────────────────────────────────────────┐
│ ◈ OpenKeet                                  Local mode   Settings │
├──────────────┬───────────────────────────────────────────────────┤
│ Practice     │ YOUR PRACTICE SPACE                               │
│ Library      │ Finish setup                                      │
│ Settings     │ Pick what to listen to and get a speech model.    │
│              │ [Start listening] [Set up audio]                  │
│              ├──────────────────────┬────────────────────────────┤
│              │ Recent sessions      │ Setup status               │
│              │ Empty until saved    │ Audio / model / context    │
└──────────────┴──────────────────────┴────────────────────────────┘
```

First-run wizard steps are **Audio source → Play test → Choose local or connect → Optional context**. Default to explicit system output selection. Show the endpoint and local processing before Start. Setup success requires a meter and a transcript, with a retry and focused troubleshooting when an endpoint fails.

## 2. Live workspace

```text
┌──────────────────────────────────────────────────────────────────┐
│ Practice session       ● Listening · 00:24    [Pause] [Stop] [↗] │
│ System output   |   Local Whisper   |   Local practice answers    │
├──────────────────────────────┬───────────────────────────────────┤
│ TRANSCRIPT                   │ CURRENT QUESTION           Rev 3  │
│ 00:08  How would you protect │ How would you protect shared state│
│        shared state          │ without blocking a coroutine?     │
│ 00:10  … without blocking   ├───────────────────────────────────┤
│        the coroutine thread? │ ANSWER                  Checked   │
│        Interim               │ Explain why a suspending mutex…   │
│ [Use full transcript]        │ [Copy]        [Session history]   │
├──────────────────────────────┴───────────────────────────────────┤
│ [Type or review a question…                         ] [Suggest] │
└──────────────────────────────────────────────────────────────────┘
```

Keep controls and prompt reachable with the keyboard. Interim text has an explicit label and separate row style. Late speech moves the answer through waiting, draft, or revised states with the revision ID visible. A final remains available in history when new work starts. Follow the transcript/answer tail only while already at the bottom. Below a narrow breakpoint, stack transcript and answer with the same heading and reading order.

**Answer view** gives the reading pane the full workspace width. Split view keeps the transcript beside it. A separate follow-up identifies the prior question while retaining both checked answers in session history. Cluely documents a transcript toggle and movable card. ParakeetAI describes an on-screen prompt that can be read at a glance. [Cluely Live Insights](https://docs.cluely.com/feature/liveinsights), [ParakeetAI](https://www.parakeet-ai.com/)

## 3. Compact focus window

```text
┌───────────────────────────────────┐
│ Focus    ● Listening [Pause] [Stop]│
│ System output · Local             │
├───────────────────────────────────┤
│ CURRENT QUESTION           Rev 3  │
│ Protect shared state without      │
│ blocking a coroutine thread?      │
│ CHECKED ANSWER                     │
│ Use a suspending mutex…           │
│ [Copy]       [Open workspace]      │
└───────────────────────────────────┘
```

Use a resizable normal window with an explicit pin control when supported. A one-line question preview expands on demand. The answer fills the remaining height, scrolls with the keyboard, and keeps copy and workspace controls visible. The window shares the active session state and capture controls. Screen sharing follows normal OS rules.

## 4. Settings and library

```text
┌──────────────────────────────────────────────────────────────────┐
│ Settings                                                         │
├────────────────┬─────────────────────────────────────────────────┤
│ Audio          │ SYSTEM OUTPUT   [Choose endpoint           ▾]   │
│ Providers      │ INPUT LEVEL     ▮▮▮▮▯▯    [Run test]             │
│ Context        │ Speech model    [Choose verified local file]    │
│ Storage        │ Locality        Audio stays on this device      │
│ Accessibility  │ [Show device health]                            │
└────────────────┴─────────────────────────────────────────────────┘
```

Providers display separate ChatGPT plan, Claude API billing, and Ollama connection states. The library owns saved sessions and local context with search, import, export, delete, and retention. Review first run, a long question with a late constraint, and compact view at 100% and 150% Windows scaling. Test readable contrast, keyboard order, scroll anchoring, and a narrow viewport before calling the UI complete.
