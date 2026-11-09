# Five interview-prep features: implementation map

This maps InstantPrep, Anticipate, TrueStory, Challenge Mode and Community Packs onto the existing OpenKeet code. It records what exists, where each feature lives, and the data boundaries. Status per feature is in [the support matrix](#support-matrix).

## Existing architecture (found, reused)

| Area | Code | Notes |
| --- | --- | --- |
| Desktop shell | `apps/desktop/src-tauri` (Tauri 2, Rust) | Windows-first. One `Desktop` state behind a mutex, 50 ms tick loop in `main.rs`. |
| Frontend | `apps/desktop/src` (React 19, TypeScript, Vite, plain CSS tokens) | Dark-first OpenKeet theme in `style.css`. Pages switched in `main.tsx`. |
| Audio + ASR | `crates/copilot-audio` (WASAPI loopback/mic, Whisper/Parakeet, bounded queues) | Emits `Utterance { id, revision, stability }` partial/final events. Not changed. |
| Question state machine | `crates/copilot-core/src/session.rs`, `boundary.rs` | Versioned turns: question `id` + `revision`, debounce, continuation/correction, follow-ups, `AnswerKey` gate that drops stale answers, cancellation flag. |
| Answer providers | `apps/desktop/src-tauri/src/providers.rs` | Ollama (local), Claude Console key, ChatGPT sign-in. Explicit consent for cloud. Not changed. |
| Local data | `history.rs`: SQLite (`rusqlite`, bundled), `user_version` migrations | Separate `sessions.sqlite3`. Not changed. |
| CLI | `apps/cli` | Hosts contributor and benchmark commands. |

The [archived Python prototype](../legacy-python.md) predates these features.

## New module boundaries

All domain logic is pure Rust in `crates/copilot-core/src/prep/` (serde only, no I/O, no network). The desktop crate adds storage and commands. The UI calls `prep` for local actions; `prep_draft_with_ai` and `prep_ai` (suggested facts, practice feedback) for user-triggered provider calls; `prep_paraphrase` to turn local embedding search on or off; and `use_prepared` to answer a forming question with a picked interpretation. Provider replies are validated in `copilot_core::prep` before anything is stored.

| Prompt service | Implementation |
| --- | --- |
| `DocumentImportService` | `prep/mod.rs` `Workspace::import_source` (text normalization, size limits), DOCX unzip in `src-tauri/src/prep.rs` (`docx_text`, size budget). |
| `ExperienceGraphService` (TrueStory) | `prep/story.rs`: rule-based claim extraction with quoted source spans, ownership qualifiers, metrics, review transitions, conflict rules, answer invalidation. |
| `PackCompilerService` (InstantPrep) | `prep/compile.rs`: JD requirement parsing (must/nice/not required), inspectable topic scores, question-family templates, evidence-aware answer variants. |
| `SearchIndex` / `AnswerResolver` | `prep/resolve.rs`: exact alias map + in-process BM25 lexical index, negation/intent/entity gates, three-way outcome. The one retrieval entry point for Session, Practice and the pack tester. |
| `QuestionStateEngine` (Anticipate) | Existing `Session` stays the turn engine. `prep/resolve.rs` adds `anticipate()` (tentative candidates keyed by question id, revision and pack revision) and `answer_from_pack()` (answers due work locally before any provider call). |
| `InterviewSimulator` + `ReadinessService` (Challenge Mode) | `prep/practice.rs`: question selection, deterministic rubric, follow-up engine, readiness with uncertainty, simple interval schedule. |
| `CommunityPackService` | `prep/community.rs`: strict schema types, validation limits, install/update/uninstall, export allowlist. |
| `AIProviderService` | Existing `ProviderHub`. InstantPrep uses it only when the user presses **Draft with AI** on one answer. |
| `LocalDataLifecycle` | `Workspace::delete_*`, private backup export, `prep.sqlite3` delete-all. |

## Data flow

```text
Resume / JD / notes (paste, TXT, MD, DOCX)
  -> Workspace::import_source -> SourceDoc (normalized text, fingerprint)
  -> story::extract -> Claim (quote + byte span, extracted_unreviewed)
  -> user review (confirm / correct / reject) -> conflicts recomputed, dependent answers -> needs_review
Installed community packs (validated JSON) ------------------+
  -> compile::compile(pack settings) -> InterviewPack revision N, QuestionFamily, AnswerVariant
  -> resolve::Resolver (built per pack revision)
       <- Session question (final revision) via answer_from_pack (no LLM on approved hit)
       <- Session question (still forming) via anticipate (tentative, never shown as an answer)
       <- Practice prompts and the "Test a question" box
Practice attempts -> rubric + follow-up -> StudyState -> recommendations
  -> "Save improved explanation" -> new AnswerVariant draft (review required)
```

## Storage and migrations

`prep.sqlite3` in the app data folder, `PRAGMA user_version = 2`, created or migrated inside an immediate transaction. Version 2 adds the `embeddings` cache table (model, text hash, vector); a version-1 database upgrades in place with its rows kept. It refuses newer versions, like `sessions.sqlite3`. Tables keep relational keys as columns and the remaining fields as a JSON payload:

`sources`, `claims`, `claim_sources` (FK cascade), `conflicts`, `stories`, `packs`, `question_families` (FK cascade to packs), `answer_variants` (FK cascade to families), `answer_claims`, `community_packs` (public scope, separate table), `practice_attempts`, `study_states`, `embeddings` (derived vectors for the active pack only), `meta` (practice state, active pack, embedding model).

Every mutation applies the change to a copy of the in-memory `Workspace`, compares it with the stored state, and writes only the changed and removed rows in one transaction. The shared in-memory copy is replaced only after commit; reads use it without copying. A failed compile or crash never leaves a half-written pack. Existing `sessions.sqlite3`, provider settings and speech models are untouched.

## Invalidation rules

- Claim corrected or rejected: answers listing it become `needs_review`; approval does not override. Index revision bumps.
- Source deleted: claims sourced only by it are deleted with their quotes; dependent answers become `needs_review`; packs built from it become `outdated`.
- Answer edited or approved: answer revision and `content_hash` change; pack revision bumps; the session resolver is rebuilt.
- Community pack updated: removed question ids are reported; private families referencing them become `needs_review`; practice history keyed by stable ids is kept.
- Recompile: approved or user-edited answers with the same id are kept; others are regenerated.

## Public/private boundary

Private: sources, claims, stories, private packs, answers, attempts. Public: `community_packs` rows only. `community::export_public` builds output from an explicit allowlist (generic conceptual questions, paraphrases, outlines, community attribution). A sentinel test puts unique strings in every private field and checks the export.

## Build sequence used

contracts and storage -> TrueStory -> Community Pack schema -> InstantPrep compiler and resolver -> Anticipate hook in `workspace.rs` -> Challenge Mode -> UI -> tests, benchmark and docs.

## Support matrix

| Capability | Status |
| --- | --- |
| Paste/TXT/MD/DOCX import, claim review, conflicts, stories | Available |
| PDF import | Available: text PDFs with page numbers; no OCR |
| Deterministic pack compile, topic scores, three answer lengths | Available |
| AI-drafted answers | Requires configured AI (one answer at a time, user-triggered) |
| Local retrieval (exact + lexical) | Available |
| Semantic/embedding retrieval | Optional: local Ollama embeddings, fused with lexical ranking |
| Prepared answers in live sessions, tentative candidates you can pick | Available |
| Speaker diarization | Not used by design: one source per session, speaker unknown |
| Typed practice, deterministic feedback, readiness, schedule | Available |
| AI-evaluated practice feedback, voice answers in Practice | Available (AI feedback needs a connected provider) |
| Community pack install/update/uninstall/export, CLI validator | Available |
| Online pack catalog, signatures | Out of scope: bundled catalog and GitHub pull requests instead |
