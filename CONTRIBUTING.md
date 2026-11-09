# Contribute to OpenKeet

**Thank you for helping make interview preparation more accessible.**

You don't need to be an expert to contribute to OpenKeet. Helpful contributions include code, documentation, testing, accessibility improvements, bug reports, and thoughtful suggestions from people preparing for interviews.

## Find a starting point

- Try the [first-run setup](docs/getting-started.md) or ask a practice question. Tell us where the instructions or keyboard navigation get confusing.
- Look for [good first issues](https://github.com/Jigsaw777/OpenKeet/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22). If the list is empty, check the [roadmap](docs/roadmap.md) for a small test or documentation gap.
- Use a [GitHub issue form](https://github.com/Jigsaw777/OpenKeet/issues/new/choose) for a reproducible bug, a feature proposal, or a documentation correction. Start with the problem someone experiences.

## Run and test locally

Clone the [OpenKeet repository](https://github.com/Jigsaw777/OpenKeet) with an account that can access it. Use your own fork for a public contribution. Start new work from current `main`.

```powershell
git clone https://github.com/Jigsaw777/OpenKeet.git
Set-Location OpenKeet
```

Install the [Windows desktop prerequisites](docs/desktop-slice.md#build-and-test), including Node.js 24 and the Rust/MSVC and libclang toolchain.

1. From the root, run the portable Rust tests.

   ```powershell
   cargo test --locked -p copilot-core -p copilot
   cargo fmt --all --check
   ```
   If you change dependencies or workflows, also run `cargo deny check` and `zizmor .github` (both install with `cargo install --locked`).

2. Start the desktop app from `apps/desktop`.

   ```powershell
   Set-Location apps/desktop
   npm ci
   npm run desktop -- dev
   ```

3. In that directory, use `npm test` for the interface protocol and `npm run build` for TypeScript and the frontend build. The [desktop guide](docs/desktop-slice.md) covers the installer and native WebView2 smoke test.

| Where | What to change |
| --- | --- |
| [Desktop UI](apps/desktop/src) | Session, model, audio, and history screens |
| [Windows backend](apps/desktop/src-tauri/src) | Tauri commands, connections, local history, and windows |
| [Session engine](crates/copilot-core) | Question detection, answers, and session state |
| [Audio engine](crates/copilot-audio) and [CLI](apps/cli) | Local transcription, Windows capture, and fixture commands |
| [Documentation](docs) and [benchmarks](benchmarks) | Guides, synthetic audio fixtures, and measurements |

## Open a pull request

1. Use a focused branch and open a PR to `main`. Follow the [branching strategy](docs/BRANCHING_STRATEGY.md) for naming, required checks, and release changes. Format Rust with `cargo fmt` and use actual button names in guides. [Sign your commits](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits); `main` only accepts PRs whose commits all have verified signatures.
2. Include the user impact, related issue, commands run, and any changes to data handling. Use synthetic text in screenshots and test data.
3. Check your diff for keys, private transcripts, and local build files. Report vulnerabilities through [SECURITY.md](SECURITY.md).

The maintainer reviews incoming work in batches. A reproducible problem and a focused diff make review easier. Share any check you could not run. Follow the [Code of Conduct](CODE_OF_CONDUCT.md).
