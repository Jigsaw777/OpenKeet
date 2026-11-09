# Private release readiness

## Initial source and history review

- Gitleaks 8.30.1 Windows x64 was verified against its release checksum. `gitleaks git --redact=100 --log-opts="--all --reflog" --no-banner .` scanned six commits at baseline `69616f4` and reported zero matches. Four dangling Git blobs were scanned individually via `gitleaks stdin --redact=100 --no-banner`, also with zero matches. [Gitleaks release](https://github.com/gitleaks/gitleaks/releases/tag/v8.30.1)
- `git log --all --format='' --name-only` showed source, docs, tests, and launcher files. `.venv`, local profiles, generated recordings, and private connection data are excluded by ignore rules. This is a pattern-based baseline check. Run it again on the release candidate and manually review source, artifacts, model rights, and history before publication.
- The root license is MIT. Keep project notices and review third-party notices separately. The archived Python inventory includes `faster-whisper` and CTranslate2 (MIT), `SoundCard` and NumPy (BSD-3-Clause), `sounddevice` (MIT), and `pypdf` (BSD-3-Clause). Binary wheels may ship extra notices, including NumPy's numeric libraries. Keep `.venv` out of distributed source and packages. [NumPy license](https://github.com/numpy/numpy/blob/main/LICENSE.txt), [sounddevice license](https://github.com/spatialaudio/python-sounddevice/blob/master/LICENSE), [pypdf license](https://github.com/py-pdf/pypdf/blob/main/LICENSE)

## Preview dependencies and speech weights

| Component | Initial license inventory |
| --- | --- |
| Rust domain and utilities | Project MIT. Serde, `serde_json`, `sha2` MIT or Apache-2.0. `hound` Apache-2.0. |
| Windows capture | `wasapi` MIT and Microsoft `windows` MIT or Apache-2.0. Windows and WebView2 are installed runtime components with their own terms. |
| Native VAD and inference | `webrtc-vad` MIT wrapper with libfvad/WebRTC BSD-3-Clause notices and patent terms. `whisper-rs` Unlicense wrapper over MIT whisper.cpp. [libfvad license](https://github.com/dpirch/libfvad/blob/master/LICENSE), [whisper-rs](https://crates.io/crates/whisper-rs) |
| Desktop | Tauri core, CLI, and dialog plugin MIT or Apache-2.0. React and Vite MIT. TypeScript Apache-2.0. |
| Optional Parakeet engine | `sherpa-onnx` 2.0.1 crate and `sherpa-onnx-c-api.dll` Apache-2.0. `onnxruntime.dll` MIT. The installer bundles both DLLs from `sherpa-onnx-v2.0.1-win-x64-shared-MT-Release-lib.tar.bz2`, SHA-256 `afa4de3ca4ab0afd53bb4aab40d28ee15608364b549ffc0fe0caf6489bfc39a5`, matching the GitHub release digest. Ship both notices. [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx), [ONNX Runtime license](https://github.com/microsoft/onnxruntime/blob/main/LICENSE) |
| Parakeet model files | NVIDIA Parakeet TDT 0.6B v3 weights are CC BY 4.0 and need attribution. The sherpa-onnx INT8 export is four files totalling 670,478,772 bytes. It is an explicit local download pinned to revision `2bda32ec70b097a55adaa07d9a7173915b43cc78`. [NVIDIA model card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3), [Pinned export](https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/tree/2bda32ec70b097a55adaa07d9a7173915b43cc78) |
| Whisper model files | OpenAI Whisper weights are MIT. `ggml-small.en.bin` is 487,614,201 bytes and `ggml-large-v3-turbo-q5_0.bin` is 574,041,195 bytes. Both are explicit local downloads. [OpenAI Whisper license](https://github.com/openai/whisper#license), [Pinned model artifacts](https://huggingface.co/ggerganov/whisper.cpp/tree/5359861c739e955e79d9a303bcbc70fb988958b1) |
| Corpus | Repository-authored text recipe and locally generated SAPI audio. The package contains the text recipe only. Review attribution and redistribution independently before introducing third-party datasets or voice samples. |

## Release gates

These steps apply to packaged releases. See the [public release checklist](OPEN_SOURCE_RELEASE_CHECKLIST.md) for the source preview and announcement gates.

1. Validate install/uninstall on a clean Windows x64 user account with no Python setup. Record build inputs, license inventory, artifact checksum, and signing status.
2. Complete the headset/device-change, scaling, keyboard, storage/export/delete, and consent checks. Capture test artifacts using synthetic data.
3. Produce the SBOM, run dependency advisories and license review against locked artifacts, repeat the redacted history scan, and inspect the packaged files.
4. Request the owner's approval for publishing the installer or any paid activation after the checks are available.

Community local use can stay free under MIT. GitHub Sponsors can support maintenance after checking the founder's India eligibility, payout method, and any fiscal host requirements. Hosted speech or sync is a later opt-in service. For illustration, if a proposed managed plan nets ₹300/month per subscriber and costs ₹120 in metered service and variable support, its contribution is ₹180. At ₹18,000/month in fixed costs that needs 100 subscribers before taxes and acquisition costs. These are placeholders for a cost model. Provider approval, privacy review, usage caps, and observed demand gate any pilot. [GitHub Sponsors eligibility](https://docs.github.com/en/sponsors/getting-started-with-github-sponsors/about-github-sponsors)

## 3.0 private Windows preview

The local NSIS install, smoke and uninstall passed. The package is unsigned. The consent gate, local-only practice responder, and explicit SQLite save and delete paths are in use. Keep the per-user history and speech models outside the release package. [Validation](validation/DESKTOP_3_0.md), [data boundaries](architecture/DESKTOP_3_0.md)

The dependency audits reported zero vulnerabilities. RustSec flagged unmaintained `proc-macro-error` 1.0.4 and an unsoundness advisory in `glib` 0.18.5 in the cross-platform lockfile. Both are outside the Windows dependency graph. Recheck before any cross-platform release. Collect the full third-party notices and SBOM before distribution. Keep Windows signing identity and timestamping in the authorized release workflow. [Dependency review](audit/DEPENDENCY_AND_LICENSE_AUDIT.md), [RustSec RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html), [RustSec RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)

Provider authorization, retention controls, physical devices and the remaining feature parity gates are tracked in the audit. Publication remains subject to owner approval. [Provider authorization](audit/PROVIDER_AUTHORIZATION.md), [feature matrix](audit/FEATURE_PARITY_MATRIX.md)
