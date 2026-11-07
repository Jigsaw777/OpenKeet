# Contribute to OpenKeet

**Thank you for helping make interview preparation more accessible.**

You don't need to be an expert to contribute to OpenKeet. Helpful contributions include code, documentation, testing, accessibility improvements, bug reports, and thoughtful suggestions from people preparing for interviews.

## Find a starting point

- Try the offline sample and tell us where the instructions or keyboard navigation get confusing.
- Look for [good first issues](https://github.com/Jigsaw777/Personal-Interview-Assistant/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22). If the list is empty, check the [roadmap](docs/roadmap.md) for a small test or documentation gap.
- Use a [GitHub issue form](https://github.com/Jigsaw777/Personal-Interview-Assistant/issues/new/choose) for a reproducible bug, a feature proposal, or a documentation correction. Start with the problem someone experiences.

## Run and test locally

Install the [Windows desktop prerequisites](docs/desktop-slice.md#build-and-test), including Node.js 24 and the Rust/MSVC and libclang toolchain.

1. From the root, run the portable Rust tests.

   ```powershell
   cargo test --locked -p copilot-core -p copilot
   cargo fmt --all --check
   ```

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
| [Session engine](crates/copilot-core) | Question detection, answers, and replay |
| [Audio engine](crates/copilot-audio) and [CLI](apps/cli) | Local transcription, Windows capture, and fixture commands |
| [Documentation](docs) and [benchmarks](benchmarks) | Guides, synthetic audio fixtures, and measurements |

## Open a pull request

1. Keep the change focused and follow the surrounding style. Format Rust with `cargo fmt`. Use task-focused language and actual button names in guides.
2. Include the user impact, related issue, commands run, and any changes to data handling. Use synthetic text in screenshots and test data.
3. Check your diff for keys, private transcripts, and local build files. Report vulnerabilities through [SECURITY.md](SECURITY.md).

Reviewers look for a reproducible problem, a focused change, and evidence that the affected workflow still works. They may ask for a narrower diff or another check. Follow the [Code of Conduct](CODE_OF_CONDUCT.md).
