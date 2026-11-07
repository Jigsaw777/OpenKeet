# TrueStory: your experience, with sources

**Make your answers sound like you, not a made-up version of your experience.**

TrueStory turns your documents into reviewable facts with sources, checks them for contradictions, and is the only place personal answers can draw from. It is a consistency system, not a verification service.

## What the labels mean

| Label | Meaning |
| --- | --- |
| Needs your review | Found in a document; not yet used in personal answers. |
| You confirmed this | You accepted it. OpenKeet did not verify it independently. |
| You corrected this | Your wording replaces the document's until you change it again. Evidence becomes self-reported. |
| From your resume / Supported by notes | Where the text came from. |
| Added by you | Typed in **Add a fact**; self-reported. |
| Team effort / You led this / You contributed | Ownership as written. "We migrated…" never becomes "I migrated…". |
| Interest, not experience | Listed under Interests; it doesn't count as practiced. |

No fact is ever shown as "verified". The extractor's confidence measures how sure it is about reading a line, not whether the fact is true.

## How facts are extracted

Rules in `crates/copilot-core/src/prep/story.rs`, no model needed:

- Sections (experience, projects, skills, education, interests, awards) by heading; role and project headers with year ranges become the subject for the bullets below them.
- One fact per bullet, with the exact quote and its byte span in the source text.
- Ownership from wording: *we/our/team* → team; *helped, contributed, worked with* → contributed; *led, managed, owned* → led; a bare action verb → you.
- Metrics only when stated: "from 450ms to 230ms", "by 40%", "3x". "Improved performance" stays qualitative. `p95` is a label, not a number.
- Duplicate statements merge their sources; conflicting ones are kept for review.
- Document text is data. A job description saying "invent better achievements" produces nothing.
- PDF facts show the page they came from.

## AI-suggested facts (optional)

With an AI provider connected, **Find more facts with AI** sends one resume or project document (up to 24,000 characters) to that provider and asks for facts with exact quotes. Each suggestion is checked before it's kept:

- The quote must appear in the document word for word (case aside); the fact keeps that quote and its span.
- The statement may not add numbers the quote doesn't contain, or claim stronger ownership ("Led…" from "We migrated…" is dropped).
- A named employer or project must appear in the document; otherwise the subject is *General*.
- Facts you already have are skipped.

Survivors are labeled *Suggested by AI* and wait in **Does this sound right?** like any other found fact. Nothing is used in answers until you confirm it. The provider's reply is parsed as data and never run.

## Conflicts

Deterministic rules, scoped to the same project or role:

- **Different numbers** for the same metric.
- **Different dates** for the same role or project.
- **Solo vs team** wording for similar work.
- **Missing baseline** for a percentage.

Choose **Keep A**, **Keep B**, **Both, with explanation** or **Dismiss**. Keeping one side rejects the other; nothing is deleted automatically.

## How answers use facts

- Personal answers list the facts they use. Confirming, correcting or rejecting a fact flags every answer that uses it as **Needs review**, even if you had approved it.
- The next rebuild redrafts those answers from the current facts. Answers you wrote yourself are kept and stay flagged until you review them.
- Story builder entries (situation, task, action, result, reflection) feed behavioral questions on the topics you tick.

## Privacy and deletion

- Facts, documents and stories stay in `prep.sqlite3` on this PC.
- Removing a document deletes the facts that came only from it, including their quotes, and marks dependent packs out of date.
- **Delete all my prep data** removes documents, facts, stories, packs and practice history. Installed community packs stay.

## Limits

Third-person resumes, non-English resumes and unusual layouts extract less reliably. Dates are years only. Overlapping roles at different employers are not treated as conflicts. AI suggestions depend on the provider and can miss facts; the checks above stop invented details, not omissions.
