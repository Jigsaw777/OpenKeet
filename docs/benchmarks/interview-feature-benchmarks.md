# Interview-prep benchmarks

Measured on 8 November 2026 IST with `copilot bench-prep`. Reproduce:

```powershell
cargo run --release -p copilot -- bench-prep 1000 --json target/bench-prep.json
```

## Environment

| | |
| --- | --- |
| CPU | 13th Gen Intel Core i5-1335U (10 cores), laptop power settings as found |
| RAM | 15.7 GB |
| OS | Windows 11 Home 10.0.26200 |
| Build | Release, rustc 1.97.1 |
| Retrieval | Exact alias + IDF-weighted lexical overlap, in-process. No embeddings, no network. Optional embedding search (local Ollama) was not benchmarked: no embedding model was installed on the test machine. |

## Results

| Measure | Target | Measured |
| --- | --- | --- |
| Warm retrieval p50, 1,000 families | < 30 ms | 0.42 ms |
| Warm retrieval p95 | < 100 ms | 0.92 ms |
| Warm retrieval p99 | | 1.27 ms |
| Index build, 1,000 families (~4,000 wordings) | | 42 ms |
| Resume import + claim extraction (short synthetic resume) | | 0.07 ms |
| Pack compile (Java pack + resume, 14 families) | | 0.59 ms |
| Remote provider requests on prepared/offline paths | 0 | 0 |
| Top-1 shown accuracy (synthetic paraphrases) | | 100% |
| Top-3 recall | | 100% |
| Wrong answer shown | 0% | 0% |
| False match on off-topic questions | | 0% |

4,500 timed queries (3 passes over 1,500 labeled queries) after 50 warm-up queries. Timing starts from question text, so it excludes speech recognition and the session's debounce. Speech timing is unchanged; see [speech benchmarks](interview-stress.md).

## Read these numbers carefully

- The synthetic pack gives every topic a unique term, so accuracy is optimistic. It shows the gates work (every "why shouldn't I use X" query reached the avoid family, never the use family), not real-world recall.
- Real-wording accuracy is covered by fixture tests instead: negation, entity swaps, corrections, Hindi-English and ambiguity cases in `crates/copilot-core/tests/prep.rs`. One correction case ("…I meant PostgreSQL, not MongoDB") lands as *Clarify* with only the PostgreSQL question offered, not as a confident answer.
- Not measured here: resident memory during compile or long sessions, disk size per pack, cold-start latency, and stage latency inside the running app. The replay test bounds an hour of synthetic speech to 256 transcript rows and 32 history entries.
- No Python baseline exists for these features, so before/after comparison is N/A.
