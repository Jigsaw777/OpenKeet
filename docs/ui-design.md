# Personal Copilot desktop wireframes

Use a compact, ink-and-teal workspace with a light default canvas, warm surfaces, and a matching dark theme. Instrument labels and times use tabular numerals. Use a small set of reusable controls with visible focus rings, normal desktop windows, and explicit source and locality labels. A distinct timeline presents speech and answer revisions without moving the question box on every partial.

Product references inform separate setup and live work, a compact session view, a persistent prompt, and accessible transcript/answer navigation. The design uses original type, layout, icons, and copy. [Cluely Live Insights](https://docs.cluely.com/feature/liveinsights), [ParakeetAI](https://www.parakeet-ai.com/)

## 1. Dashboard and first run

```text
┌──────────────────────────────────────────────────────────────────┐
│ ◈ Personal Copilot                          Local mode   Settings │
├──────────────┬───────────────────────────────────────────────────┤
│ Practice     │ YOUR PRACTICE SPACE                               │
│ Library      │ Start with a rehearsal                            │
│ Settings     │ Try a guided sample or set up your audio.           │
│              │ [Try sample session] [Set up audio]               │
│              ├──────────────────────┬────────────────────────────┤
│              │ Recent sessions      │ Setup status               │
│              │ Empty until saved    │ Audio / model / context    │
└──────────────┴──────────────────────┴────────────────────────────┘
```

First-run wizard steps are **Audio source → Play test → Choose local or connect → Optional context**. Default to explicit system output selection. Show the endpoint and local processing before Start. The sample path opens directly without permission prompts, account creation, or a model download. Setup success requires a meter and sample transcript, with a retry and focused troubleshooting when an endpoint fails.

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

Use a resizable normal window with an explicit pin control when supported. It shares the active session state and capture controls. Screen sharing follows normal OS rules. No separate recording or generation task starts when opening it.

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
