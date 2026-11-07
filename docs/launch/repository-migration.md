# OpenKeet repository migration

Audit date: 7 November 2026. The authenticated account was `Jigsaw777` with admin access. The existing private repository has GitHub repository ID `1445710170`, default branch `main`, and an MIT source license. The owner already used this repository for the Windows desktop app and its reviewed changes.

| | Repository |
| --- | --- |
| Previous URL | [Jigsaw777/Personal-Interview-Assistant](https://github.com/Jigsaw777/Personal-Interview-Assistant) |
| Verified canonical URL | [Jigsaw777/OpenKeet](https://github.com/Jigsaw777/OpenKeet) |
| Preflight | The canonical name returned 404 under the authenticated owner before the rename. |

## What could be affected

- GitHub advertised three branches and no tags. PRs #1 and #2 were merged, with no open PRs. The legacy branch refs are still useful as history. The local checkout also contained unpublished interview-prep work.
- The old URL appeared in the README, contribution and support documents, and the issue form chooser. Desktop metadata already used the OpenKeet executable and `dev.personalcopilot.desktop` as its application ID.
- GitHub reported no releases, Pages site, webhooks, deploy keys, deployments, Actions secrets, or Actions variables. The tracked workflows build preview artifacts on demand. The app has no configured updater or repository URL in its executable metadata.
- GitHub returned HTTP 403 for branch protection and rulesets on this private repository. Actions were enabled with a read-only default token. Private fork workflows, fork secrets, and fork write tokens were disabled.

## Verification and rollback

The in-place rename preserved the repository ID, private visibility, `main` as the default, and all three advertised refs. Git operations at the old URL advertised the same refs as the new URL. The local `origin` and tracked project links now use the canonical address. GitHub reports the updated description and docs URL, the additional `ai-assistant` topic, and a squash commit message based on the PR title and body.

GitHub redirects existing Git traffic and repository links while the old name stays available. Links embedded in other projects or services may still need updating. If rollback is required, restore the original repository name while it is available, update `origin`, and revert URL changes with a new PR. [GitHub rename behavior](https://docs.github.com/en/repositories/creating-and-managing-repositories/renaming-a-repository)
