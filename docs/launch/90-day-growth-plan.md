# OpenKeet's first 90 days

Day 1 starts after owner approval and a verified public Windows release. Before that, work through the [release checklist](../OPEN_SOURCE_RELEASE_CHECKLIST.md). This plan assumes one maintainer, no advertising budget, no hosted inference service, and about three hours per week for launch and community work. Those are planning assumptions.

| Stage | Do first | Evidence to collect |
| --- | --- | --- |
| Before day 1 | Verify the installer and rights. Run three clean setup trials if testers are available. Prepare two or three reproducible starter issues, a captioned demo, and the public links. | First-run notes, signing and checksums, approved copy, and a private security route |
| Days 1–14 | Start with the GitHub Release, Discussions, and one LinkedIn post. Choose one eligible Reddit venue once you can respond. Answer real questions and fix the first repeated setup blocker. | Attempts to install, answer-model and device failures, feedback that led to an issue |
| Days 15–30 | Publish the implementation article or a technical demo. Bring the same fixture and reproducible results to one appropriate technical community. Triage and label issues weekly. | Reproducible reports, documentation corrections, contributor questions |
| Days 31–60 | Ship a validated fix release if warranted. Credit contributors with their consent. Add a small fixture or contributor walkthrough for the most frequent question. Explore pack contributions only after an actual interface is ready. | First accepted external contributions, fixed setup categories, follow-up feedback |
| Days 61–90 | Review what drove installs and useful reports. Fix the most common friction, share a concise progress demo, and drop channels that consume time without learning. | Repeat contributors, change in setup failures, a short public progress summary |

Start with one original launch post. Space further posts according to each community's rules and the maintainer's response capacity. An engineering follow-up should have new evidence, such as a comparison fixture or a shipped fix. [Platform research](platform-research.md)

## Measure without adding app telemetry

Record a weekly snapshot. Use a private table with the date, release tag, channel, metric, and what changed that week.

| Signal | How to record it | How to interpret it |
| --- | --- | --- |
| Visits, full clones, stars, forks | Capture GitHub traffic and weekly count changes. The traffic view uses a rolling 14-day window. | Interest and source access. These signals don't identify individual adopters. [GitHub traffic](https://docs.github.com/en/repositories/viewing-activity-and-data-for-your-repository/viewing-traffic-to-a-repository) |
| Downloads and first run | Record GitHub release asset download totals. Invite optional, anonymous-to-the-app setup reports with Windows version, tag, and where setup stopped. | A download is a fetch, and a setup report is self-selected. Use categories to find repeated friction. [GitHub release assets](https://docs.github.com/en/rest/releases/assets) |
| Reliability | Count reproducible crashes and report categories in Issues. Ask for redacted logs only when needed. | Distinguish reports from confirmed defects. Collect an opt-in install or crash metric only if an approved consent flow is added later. |
| Contribution | Track opened and merged PRs, outside contributors, first review time, and repeat contributors. Count docs changes and maintained packs when those workflows exist. | A small number of useful contributions can be a strong result. |
| Community | Count discussions that lead to a documented answer, validated report, or decision. | Prioritize reports that change the product or guide. |

Use the public release link as the primary destination. For attribution, prefer GitHub referrers and a voluntary “Where did you hear about OpenKeet?” question. Use source labels in links only if a future approved site can aggregate them without storing personal data. Avoid tracking pixels, fingerprinting, and collection of interview content.

Planning targets for the first two weeks: aim to learn from five attempted setups, resolve one repeated blocker, and get feedback on one starter issue. If fewer people respond, report that honestly and improve onboarding before widening promotion.
