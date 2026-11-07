# InstantPrep: interview packs

**Tell us about the role. We'll help you prepare answers that fit your experience.**

An interview pack is a private, local set of practice questions and answers built from your resume, a job description, optional project notes and any community packs you choose. OpenKeet doesn't guess an employer's exact questions.

## Build a pack

1. Open **Prepare**. Add your resume and the job description: paste text, or choose a PDF, DOCX, TXT or Markdown file. PDF text keeps its page numbers, so each fact shows where it came from. Pages with no text (scanned images) are listed in a warning; OpenKeet doesn't run OCR, so paste their text. Password-protected PDFs are refused with a message.
2. Check the extracted text preview and the job summary (must have, nice to have, not required).
3. Review the facts OpenKeet found in **My Experience**. Unreviewed facts can choose topics but never appear as claims about you.
4. Choose a depth (up to 4, 8 or 14 topics) and optional community packs, then **Build interview pack**.
5. Open a question to edit, approve, exclude or draft it. Each has a 15-second, 45-second and 2-minute version.
6. **Try a question** shows whether an answer would come from your approved pack, a community concept, or nowhere.
7. **Use in sessions** makes approved answers appear instantly in live and typed sessions, labeled *From your prepared pack*.

## How topics are ranked

Each topic shows its score parts and a sentence explaining them, for example: *Prioritized because the job description asks for Concurrency, and you confirmed related experience, and 5 community questions are available.*

| Part | Weight | Value |
| --- | --- | --- |
| Job description | 0.45 | required 1.0, preferred 0.6, mentioned 0.5, "not required" 0 |
| Your experience | 0.25 | confirmed 1.0, unreviewed 0.5, listed as an interest 0.2 |
| Gap to practice | 0.15 | 1 when the job asks for it and you haven't confirmed experience |
| Community questions | 0.15 | up to 5 questions available |

The score orders practice. It is not a hiring-fit percentage.

## Questions and answers

- Each topic gets question families from templates (what it is, when you'd avoid it, troubleshooting, your experience) or from community packs, which replace matching templates. "When would you use X" and "when would you avoid X" are separate families.
- **Personal answers** are drafted only from facts you confirmed or corrected, keeping ownership as stated ("We migrated…" stays team work). Numbers come only from confirmed facts. With no confirmed facts, the answer stays empty with a prompt such as *What changed after the work?*
- **Community answers** come from a pack's reference answer and outline, labeled *Community concept* with author and license.
- **Concept questions** without community material start empty. Write the answer, or use **Draft with AI** if you connected a provider. That sends the question, its outline and your confirmed facts for that topic to the provider. The result is always a draft, and numbers missing from your confirmed facts are flagged.
- **Draft N answers with AI** drafts every question that has no answer you approved, wrote or already drafted, one request at a time. **Stop after** ends it between questions. Community reference answers are left alone.
- Every generated answer starts as a **Draft**. Only **Approved** answers are served as *Prepared answers*.

## Retrieval

`AnswerResolver` (`crates/copilot-core/src/prep/resolve.rs`) is the one retrieval path for sessions, practice and the tester:

1. Normalize text: lowercase, contractions (`don't` → `do not`), plural stems, common Hindi-English function words. Technical tokens such as `C++`, `p95` and `ConcurrentHashMap` stay intact.
2. Extract features: content tokens, intent (explain, compare, debug, design, experience), negation ("avoid", "not", "downsides"; "avoid a deadlock" counts as prevention, not negation), entities from the skill vocabulary, and self-corrections ("I meant PostgreSQL, not MongoDB").
3. Exact alias match, else an IDF-weighted token-overlap score in [0, 1] over every question and paraphrase.
4. Gates: negation must match, comparison and personal questions need matching intents, entities must overlap with no swap inside the same area (Redis vs Memcached).
5. Outcome: *Prepared* (approved, score ≥ 0.6, no rival within 0.08), *Community*, *Unapproved*, *Clarify* (close rivals or score 0.3–0.6), or *No reliable match*.

A prepared or community hit never calls a model. Thresholds were set from the fixtures in `crates/copilot-core/tests/prep.rs`.

### Catch reworded questions (optional)

Under a pack that's used in sessions, **Catch reworded questions** turns on embedding search with a local Ollama embedding model (for example `nomic-embed-text`). OpenKeet embeds every question wording once, caches the vectors in the local database (schema version 2) and keeps only the active pack's vectors.

- Wording matching always runs first. Only when it finds no reliable answer does a live question get embedded, with a 1.5-second limit and an in-memory cache for repeated questions.
- The two rankings are fused by reciprocal rank (k = 60) for the candidate list. An embedding-only answer needs cosine ≥ 0.82, no rival within 0.04, and no conflicting wording match. The negation, intent and entity gates still decide, so "when to avoid Redis" never gets the "when to use Redis" answer.
- If Ollama isn't running, sessions keep wording matching. **Try a question** uses wording matching only.
- The cosine thresholds are not calibrated per embedding model. Check a few paraphrases after switching models.

## Changes and recovery

- Editing or approving an answer creates a new revision and bumps the pack revision; sessions rebuild their index.
- Changing a document, fact, story or community pack marks dependent packs **Out of date**. **Rebuild** regenerates drafts and keeps answers you wrote. Approved drafts whose text changed need approval again.
- Builds happen in memory and are saved in one SQLite transaction. A failure keeps the last good pack.
- **Delete pack** removes its questions, answers and practice history without touching other packs.

## Limits

No OCR for scanned PDFs, no pack duplicate or archive. Embedding search uses local Ollama only, never a cloud provider. See the [benchmark report](../benchmarks/interview-feature-benchmarks.md) for measured speed.
