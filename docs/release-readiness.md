# Private release readiness

## Initial source and history review

- Gitleaks 8.30.1 Windows x64 was verified against its release checksum. `gitleaks git --redact=100 --log-opts="--all --reflog" --no-banner .` scanned six commits at baseline `69616f4` and reported zero matches. Four dangling Git blobs were scanned individually via `gitleaks stdin --redact=100 --no-banner`, also with zero matches. [Gitleaks release](https://github.com/gitleaks/gitleaks/releases/tag/v8.30.1)
- `git log --all --format='' --name-only` showed source, docs, tests, and launcher files. `.venv`, local profiles, generated recordings, and private connection data are excluded by ignore rules. This is a pattern-based baseline check. Run it again on the release candidate and manually review source, artifacts, model rights, and history before publication.
- The root license is MIT. Keep project notices and review third-party notices separately. The legacy direct inventory includes `faster-whisper` and CTranslate2 (MIT), `SoundCard` and NumPy (BSD-3-Clause), `sounddevice` (MIT), and `pypdf` (BSD-3-Clause). Binary wheels may ship extra notices, including NumPy's numeric libraries. Keep `.venv` out of distributed source and packages. [NumPy license](https://github.com/numpy/numpy/blob/main/LICENSE.txt), [sounddevice license](https://github.com/spatialaudio/python-sounddevice/blob/master/LICENSE), [pypdf license](https://github.com/py-pdf/pypdf/blob/main/LICENSE)

## Release gates

1. Validate install/uninstall on a clean Windows x64 user account with no Python setup. Record build inputs, license inventory, artifact checksum, and signing status.
2. Complete the headset/device-change, scaling, keyboard, storage/export/delete, and consent checks. Capture test artifacts using synthetic data.
3. Produce the SBOM, run dependency advisories and license review against locked artifacts, repeat the redacted history scan, and inspect the packaged files.
4. Request the owner's approval for public visibility, public release artifacts, or any paid activation after the checks are available.

Community local use can stay free under MIT. GitHub Sponsors can support maintenance after checking the founder's India eligibility, payout method, and any fiscal host requirements. Hosted speech or sync is a later opt-in service. For illustration, if a proposed managed plan nets ₹300/month per subscriber and costs ₹120 in metered service and variable support, its contribution is ₹180. At ₹18,000/month in fixed costs that needs 100 subscribers before taxes and acquisition costs. These are placeholders for a cost model. Provider approval, privacy review, usage caps, and observed demand gate any pilot. [GitHub Sponsors eligibility](https://docs.github.com/en/sponsors/getting-started-with-github-sponsors/about-github-sponsors)
