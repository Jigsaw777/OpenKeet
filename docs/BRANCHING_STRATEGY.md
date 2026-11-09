# OpenKeet branching and releases

OpenKeet uses GitHub Flow. `main` integrates reviewed changes. A release comes from an explicit version tag and verified assets. The [release checklist](OPEN_SOURCE_RELEASE_CHECKLIST.md) covers the public launch gates.

## Choose a branch

| Branch | Use it for |
| --- | --- |
| `main` | Reviewed, tested changes. Use a pull request for updates. |
| `feature/<issue>-<description>` | New behavior, such as `feature/101-instantprep`. |
| `fix/`, `docs/`, `refactor/`, `perf/`, `test/`, `chore/` | Focused changes. Use the same issue and description format. Omit the issue number when there is no issue. |
| `experiment/<description>` | A prototype with a clear question to answer. Open a draft PR when the experiment is ready to review. |
| `release/vX.Y.Z` or `hotfix/<version>-<description>` | Temporary stabilization or an urgent fix for a supported release. |

New development starts from current `main`. Keep branches short. The repository has one permanent integration branch. [Migration audit](launch/repository-migration.md)

## Submit and review a change

1. Start from a checkout you can push to. Outside contributors can fork the repository. Create a descriptive branch from current `main`.
2. Make one focused change. Run the relevant [local checks](../CONTRIBUTING.md) and use a clear commit message, such as `docs: improve local model setup`.
3. Open a PR against `main`. Include the user impact, a reproduction or example, and test results.
4. Resolve review feedback and update the branch to current `main`. The maintainer checks correctness, privacy impact, scope, and CI on the latest head.
5. Use **Squash and merge**. After verifying the work is merged, delete the short-lived branch if no other PR depends on it.

If your branch falls behind, merge current `main` into it when an update is needed. Preserve published history and use a follow-up PR to revert a merged change. Before cleaning up a branch, verify the PR was merged and any unique work is accounted for. A squash merge keeps the final change without preserving every feature commit in `main`.

## Checks and branch protection

Every PR to `main` or a release branch runs **Verify pull request**:

| Check | What it enforces |
| --- | --- |
| **Documentation links** | Local links and anchors in every Markdown file resolve. |
| **Rust checks** | Formatting, Clippy with warnings as errors, and the portable unit tests (Linux). |
| **Desktop frontend** | Frontend tests and build, and no high or critical production npm advisories. |
| **Windows workspace** | Clippy with warnings as errors and every test in the workspace, including the desktop app and speech engine (Windows). |
| **Supply chain** | `cargo deny`: no known vulnerable or unsound crates, only allowed licenses, crates.io as the only source. Policy and exceptions live in [deny.toml](../deny.toml). |
| **Dependency review** | The PR adds no dependency with a high or critical advisory. |
| **Workflow security** | `zizmor` finds no security issues in the workflows (template injection, credential leaks, unpinned actions). |

**CodeQL** analyzes the workflows, TypeScript and Rust on each PR, on `main`, and weekly. All jobs check out code with a read-only token and no persisted credentials, and every action is pinned to a commit. [Dependabot](../.github/dependabot.yml) proposes grouped updates monthly, capped at two open PRs per ecosystem, and security fixes as advisories appear. Use the manually dispatched **Verify desktop preview** workflow for native Windows packaging and dependency audit before a release candidate.

The [default branch ruleset](https://github.com/Jigsaw777/OpenKeet/rules) requires, on `main`:

- A PR merged with **Squash and merge**, with all conversations resolved. Stale reviews are dismissed on new pushes.
- All seven checks above, passing on a branch that is up to date with `main`.
- CodeQL results with no high or critical security alerts and no errors.
- Linear history and signed commits. GitHub signs squash merges automatically.
- No deletion and no force pushes. There are no bypass actors.

Release tags matching `v*` can be created but not moved or deleted. Community pack validation runs when its inputs change; review its result when it runs. Run `cargo deny check` and `zizmor .github` locally before changing dependencies or workflows.

The ruleset requires no separate approvals while there is one maintainer. When a second maintainer can review independently, require one approval. If a long-lived release branch becomes necessary, give it the same PR and check requirements. [GitHub ruleset settings](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/creating-rulesets-for-a-repository)

Secret scanning with push protection, Dependabot alerts and security updates, and private vulnerability reporting are on. Review security advisories and installer evidence at release time. Keep review or policy exceptions in the PR or release record.

## Tag, verify, and publish a release

1. Merge the reviewed changes. Use a candidate branch only when stabilization needs to continue separately from `main`.
2. Choose a [Semantic Versioning](https://semver.org/) tag for the verified commit. Tag approved releases explicitly, for example `vX.Y.Z` for stable or `vX.Y.Z-rc.N` for a candidate.
3. Build from the tagged source with the locked dependencies. Verify install, launch, teardown, and the supported provider paths. Record the checksums, signing status, and dependency notices.
4. Prepare a draft GitHub Release using the [release notes template](../.github/RELEASE_TEMPLATE.md). Set **Pre-release** for beta or candidate artifacts. Use stable for builds that cleared the supported Windows checks.
5. Publish approved artifacts only after the [release checklist](OPEN_SOURCE_RELEASE_CHECKLIST.md) is clear. Check the final download as a new user.

For a hotfix, start from the affected release tag or its maintenance branch when current `main` contains unrelated work. Open a focused PR, run the same checks, and tag the next patch release after verification. Bring the fix to `main` through a second PR. Backport to another supported release only when it has the same defect and the patch has been verified there.
