# Community interview packs

You don't need to change the app to improve OpenKeet. Contribute a question pack, improve a reference answer, or add a skill area.

A pack is one JSON file: questions, paraphrases, answer outlines, optional reference answers and follow-ups for one role or topic. Packs are data only. OpenKeet never runs anything in them, never fetches their links, and never sends a user's private data to pack authors.

## Write a pack

1. Copy [`examples/java-backend.json`](examples/java-backend.json) and give it a new `id` such as `org.yourname.android-kotlin` and `version` `1.0.0`.
2. List `skills`. Reuse built-in ids where they fit (`java`, `kotlin`, `concurrency`, `caching`, `redis`, `postgresql`, `databases`, `apis`, `system-design`, `observability`, `leadership`, `communication`, ...) so packs combine with a user's resume topics.
3. Add `questions`. Each needs `id`, `skillIds`, `question` and `intent` (`explain`, `compare`, `design`, `debug`, `tradeoff`, `experience`, `behavioral`, `other`). Add `paraphrases` the way interviewers actually ask, an `answerOutline` of key points, and `followupIds` that link to other questions in the pack.
4. Keep "when would you use X" and "when would you avoid X" as separate questions. OpenKeet never reuses an answer across opposite intents.
5. Validate locally:

```powershell
cargo run -p copilot -- validate-packs community-packs/examples
```

The full rules are in the [schema](../schemas/openkeet-community-pack.schema.json). The Rust validator is authoritative: it also checks that `skillIds` and `followupIds` point at real entries and rejects HTML, script links and files over 1 MB.

## Content rules

- **License.** Use `CC-BY-4.0`, `CC-BY-SA-4.0` or `CC0-1.0` for content you have the right to share. This is separate from the app's MIT code license.
- **Original and accurate.** Write in your own words. Cite official documentation in `references` for factual claims. Reference answers are starting points, not guarantees; say so where trade-offs depend on context.
- **Fair questions.** Ask what the role needs. Outlines should allow several valid answers.
- **No leaks or personal data.** Never include an employer's confidential interview questions, leaked materials, a real person's resume, stories, names or contact details.
- **Behavioral packs** have outlines and follow-ups but no reference answers. Candidates answer with their own experience.

## Pull request checklist

- [ ] `validate-packs` passes.
- [ ] Content is original or properly licensed, and references support factual claims.
- [ ] No personal data, employer-confidential questions or leaked material.
- [ ] `version` is bumped when you change an existing pack. Keep question `id`s stable so practice history carries over; removed ids are reported to users.

## Review and corrections

Maintainers review packs for accuracy, fairness and licensing. Anyone can open a correction pull request or an issue about an inaccurate answer. A pack with unresolved accuracy problems can be marked deprecated in its description or removed from the bundled examples. A valid schema means a pack is well-formed, not that every answer is correct.
