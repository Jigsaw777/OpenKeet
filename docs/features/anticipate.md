# Anticipate: prepared answers in sessions

When a session uses an interview pack, OpenKeet looks up approved answers locally as the question takes shape, and shows one only when the question is settled and the match is safe.

## How it fits the existing session engine

The question state machine in `crates/copilot-core/src/session.rs` stays the source of truth. It already has versioned turns (`question_id` + `revision`), debounce and continuation windows, corrections ("I mean…"), follow-up linking, cancellation, and an `AnswerKey` check that drops answers for an old revision. Anticipate adds two hooks in `prep/resolve.rs`, called from `Desktop` in `apps/desktop/src-tauri/src/workspace.rs`:

| Session state | Status shown | What Anticipate does |
| --- | --- | --- |
| No question | Listening | Nothing |
| `incomplete` | The question is still taking shape | `anticipate()` computes tentative candidates, shown as *Possible interpretation · not final* |
| `candidate` / `waiting` | Question heard | Same; candidates are recomputed when the revision changes |
| `ready` → work issued | Looking through your prepared answers… | `answer_from_pack()` resolves the final revision text before any provider call |
| Hit | Answer ready, labeled *From your prepared pack* or *Community concept* | Completes the work locally; no model call |
| No hit, provider connected | Writing answer… | Normal provider request, labeled *AI-generated suggestion* |
| No hit, no provider | Answer failed | "We couldn't find a reliable prepared answer…" |

Tentative candidates carry `(session_id, question_id, revision, pack_revision)`. The UI shows them only while all four match the current question and pack, so a revision or pack edit drops them. Prepared answers are resolved synchronously against the exact revision in the work item; if speech resumes, the session supersedes the answer as before. Provider answers keep the existing stale-answer gate.

The boundary rules gained one change: a request verb with no object ("Can you explain…", "Describe…") counts as incomplete, so "Can you explain… (pause) …thread pools versus virtual threads?" stays one question.

## Timing policy

Existing defaults are unchanged: 3.2 s debounce (shortened by 1/8 for short explicit questions), 5 s continuation, 120 s follow-up window. Prefetch is a lookup on every revision change, costing well under a millisecond. Silence alone never finalizes a turn while speech is still pending.

## Replay fixtures

`anticipate_replays_never_surface_an_obsolete_answer` in `crates/copilot-core/tests/prep.rs` replays timestamped transcript events without a microphone:

- "Why use Redis" revised to "…why not use Redis for strongly consistent writes?" while an answer is in flight: the in-flight work is cancelled, the tentative result is discarded, and the only answer shown is the "avoid" answer (X3, B3, B7).
- "Can you explain" + pause + "thread pools versus virtual threads?" becomes one comparison question (B4).
- Repeated interim prefixes leave no duplicated words (B5).
- A pack edit invalidates a pending tentative result (B8).
- Distinct questions after a long pause get distinct ids and are answered exactly once (B1, B6).
- An hour of synthetic speech keeps transcript and history bounded (B12).

Resolver cases in `resolver_separates_negation_entities_and_edits` cover corrections (B9), Hindi-English (B10), a quoted "why" statement (B11) and near-equal candidates (B15). Out-of-order finals (B14) are rejected by the existing session tests. Everything runs with no provider (B13).

## Pick an interpretation

While a heard question is still taking shape, each listed interpretation has **Use this answer**. It answers the current question revision with that pack question's answer at once. It still passes the negation, intent and entity gates, and if the question changes before the answer is due, the pick is dropped. If that pack question has no approved or community answer, the usual route applies: the AI provider if one is connected, otherwise *no prepared match*.

## Speakers


Each session listens to one source: your microphone or the call's playback device. OpenKeet doesn't separate voices within a source, so speech is treated as *speaker unknown*. It never assumes who asked a question. Practice works without speaker separation.
- Separate stage latency metrics in the live UI. Retrieval timing is in the [benchmark report](../benchmarks/interview-feature-benchmarks.md); speech-end to final transcript is unchanged from the [speech benchmarks](../benchmarks/interview-stress.md).

Use live help only where the interview allows it. Capture always starts from **Start listening** and can be paused or stopped.
