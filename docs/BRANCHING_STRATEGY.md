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

New development starts from current `main`. Keep branches short. The repository has one permanent integration branch. The two existing merged remote branches remain available as history. [Migration audit](launch/repository-migration.md)

## Submit and review a change

1. Start from a checkout you can push to. Once the repository is public, outside contributors can fork it. Create a descriptive branch from current `main`.
2. Make one focused change. Run the relevant [local checks](../CONTRIBUTING.md) and use a clear commit message, such as `docs: improve local model setup`.
3. Open a PR against `main`. Include the user impact, a reproduction or example, and test results.
4. Resolve review feedback and confirm the latest head passes CI. The maintainer checks correctness, privacy impact, and scope.
5. Prefer **Squash and merge** for a focused PR. After verifying the work is merged, delete the short-lived branch if no other PR depends on it.

If your branch falls behind, merge current `main` into it when an update is needed. Preserve published history and use a follow-up PR to revert a merged change. Before cleaning up a branch, verify the PR was merged and any unique work is accounted for. A squash merge keeps the final change without preserving every feature commit in `main`.

## Checks and branch protection

The PR workflow runs **Documentation links**, **Rust checks**, and **Desktop frontend** on PRs to `main` and release branches. It checks local links and anchors, Rust formatting and Clippy, portable unit tests, frontend tests and build, and high or critical production npm advisories. GitHub Actions checks out PR code with a read-only token and no persisted checkout credentials. Use the manually dispatched **Verify desktop preview** workflow for native Windows packaging and dependency audit before a release candidate.

GitHub returned HTTP 403 for branch protection and rulesets on the current private plan. Until enforcement is available, the maintainer verifies the current PR checks and resolves conversations before merging. Use these steps when the repository is eligible. [GitHub protection availability](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)

1. Open **Settings**, then **Rules**, then **Rulesets**. Create a branch ruleset targeting the default branch.
2. Restrict deletions and block force pushes. Require a pull request and resolved conversations. Set required approvals to zero while there is one maintainer.
3. Require **Documentation links**, **Rust checks**, and **Desktop frontend** once each has passed on this repository. Check that the chosen names appear on a fresh PR.
4. Activate the rule and confirm GitHub reports it for `main`. Apply the same controls to a `release/**` rule when a supported release branch needs them.

Review security advisories and installer evidence at release time. Keep review or policy exceptions in the PR or release record.

## Tag, verify, and publish a release

1. Merge the reviewed changes. Use a candidate branch only when stabilization needs to continue separately from `main`.
2. Choose a [Semantic Versioning](https://semver.org/) tag for the verified commit. Tag approved releases explicitly, for example `vX.Y.Z` for stable or `vX.Y.Z-rc.N` for a candidate.
3. Build from the tagged source with the locked dependencies. Verify install, launch, teardown, and the supported provider paths. Record the checksums, signing status, and dependency notices.
4. Prepare a draft GitHub Release using the [release notes template](../.github/RELEASE_TEMPLATE.md). Set **Pre-release** for beta or candidate artifacts. Use stable for builds that cleared the supported Windows checks.
5. Publish approved artifacts only after the [release checklist](OPEN_SOURCE_RELEASE_CHECKLIST.md) is clear. Check the final download as a new user.

For a hotfix, start from the affected release tag or its maintenance branch when current `main` contains unrelated work. Open a focused PR, run the same checks, and tag the next patch release after verification. Bring the fix to `main` through a second PR. Backport to another supported release only when it has the same defect and the patch has been verified there.
