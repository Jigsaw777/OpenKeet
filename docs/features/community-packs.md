# Community Packs

**Learn from interview preparation shared by the community. Make it your own.**

Community packs are public, data-only question sets for a role or topic. They install offline, combine with your private pack on this PC, and never receive your data.

## Use a pack

1. Open **Community**. **Explore packs** lists the starter packs included with OpenKeet: Java backend, system design, and behavioral (STAR).
2. Check the title, authors, license, version and question count, then **Install**.
3. **Import a pack** previews a `.json` file before installing. OpenKeet validates it fully; nothing is installed if any check fails, and the error names the field.
4. **Use this pack to prepare** opens Prepare with the pack selected. The resulting interview pack is private; reused community content keeps its attribution and license.
5. Installing a newer version keeps your practice history (question ids are stable). Removed questions are reported, and answers that relied on them are flagged for review. Older versions and changed content without a new version number are refused.

## Two kinds of pack

| | Public community pack | Private interview pack |
| --- | --- | --- |
| Contains | General questions, outlines, reference answers, attribution | Your documents, facts, stories, answers, practice |
| Stored in | `community_packs` table | Other tables in `prep.sqlite3` |
| Shared | Only by you, as a file you export | Never |

**Export shareable pack** (Prepare → Manage this pack) builds a community pack from an allowlist: generic question text, paraphrases, outlines and community attribution. Your answers, documents, facts, stories, names, company, role and file names are never read. A test injects unique strings into every private field and checks the export.

## Safety

- Strict schema: unknown fields, HTML tags, `javascript:`/`data:` links, oversized files (1 MB), too many items, duplicate ids, broken follow-up links and unknown licenses are rejected.
- Links must be plain `https://` URLs and are shown as text; OpenKeet never fetches them.
- Pack text is untrusted data. It can't change prompts, tools or settings. If you use **Draft with AI**, pack text is quoted as data in the request.
- A valid pack is well-formed, not necessarily correct. Reference answers are labeled *Community concept*; check them.

## Contribute

See [community-packs/README.md](../../community-packs/README.md) for authoring, the license policy and the review checklist, and the [JSON Schema](../../schemas/openkeet-community-pack.schema.json). Validate with:

```powershell
cargo run -p copilot -- validate-packs community-packs/examples
```

The `Validate community packs` workflow runs the same check and the pack tests on pull requests, with no cloud services.

## Out of scope

There's no online catalog or pack signing, by design: packs ship with the app or arrive as reviewed pull requests to `community-packs/`, with no server to run. A future signed index would show who published a pack, which is not the same as editorial review. Also not supported: ZIP containers, Markdown rendering (text is shown plain), pack-specific rubrics.
