# OpenKeet public release checklist

OpenKeet is a Windows 11 x64 source preview under the existing [MIT license](../LICENSE). Publishing source, distributing a signed installer, and announcing a launch are separate steps.

| Stage | Gate | Evidence required |
| --- | --- | --- |
| Source | Code and asset redistribution | Review inherited and adapted code, confirm the right to share artwork, and preserve license notices. [Dependency and license audit](audit/DEPENDENCY_AND_LICENSE_AUDIT.md) |
| Source | Security and privacy | Scan the published refs and review fixtures, screenshots, build logs, and endpoint configuration. Verify private vulnerability reporting and [the default branch rules](BRANCHING_STRATEGY.md). |
| Installer | Verified Windows package | Build from a tag. Verify installation and uninstall on a clean account, signing and checksum, a software bill of materials, and third-party notices. [Release readiness](release-readiness.md) |
| Installer | Provider and device evidence | Verify real provider authorization, limits and disconnection. Recheck physical mic and headset switching and capture permissions. Keep unverified behavior marked as preview. [Provider audit](audit/PROVIDER_AUTHORIZATION.md) |
| Announcements | Maintainer sign-off | Review the exact release links and community drafts. Get separate approval for announcements. |

## Security

- [x] The rename preflight and the source preview preflight used fully redacted Gitleaks scans. No matches were reported in the scanned history, candidate files, or historical Actions logs. Pattern matching is one input to the review.
- [x] GitHub reported zero repository Actions secrets and variables in the rename preflight. PR runs use a read-only token and do not persist checkout credentials.
- [ ] Before shipping an installer, review the tagged refs, fixture data, generated logs, screenshots, and package contents for personal information, debug credentials, keys, and private endpoints. Test files should contain synthetic or licensed inputs.
- [ ] Document the installer source tag, toolchain, checksum, signing, and provenance. Run dependency advisory checks against the exact lockfiles and record which findings affect the Windows package.
- [ ] Test private report delivery with the maintainer before announcing the release. Use [SECURITY.md](../SECURITY.md) for the contact route.

## Licensing and intellectual property

- [x] The root [MIT license](../LICENSE) is the current project source license. The rename keeps that declaration.
- [x] The [source audit](audit/DEPENDENCY_AND_LICENSE_AUDIT.md) separates the restricted Natively research reference from the local text bridge, which uses an Ollama-compatible protocol.
- [ ] Inventory packaged libraries, native speech runtime DLLs, installer components, and build tooling. Include their required license notices with the release.
- [ ] Review terms for model downloads, including default Whisper and experimental NVIDIA Parakeet. Keep their provenance, checksums, notices, and attribution documented. The preview downloads weights separately.
- [x] The owner supplied the logo artwork and approved publication of the source repository. Manrope is used as artwork outlines, with no font files bundled. [Brand guide](brand/BRAND.md) · [SIL OFL FAQ](https://openfontlicense.org/ofl-faq/)
- [ ] Review demo footage and newly produced screenshots before using them in a launch announcement.
- [ ] Confirm that contributions can be distributed under the repository's declared terms. Document any exception for third-party code, data, or assets. Review any license change separately.

## Product and community

- [x] OpenKeet repository name, topic metadata, candidate-focused README, and support links have a canonical destination. [Migration audit](launch/repository-migration.md)
- [x] The docs cover Windows source installation, local transcription, answer-provider setup and billing, and [data flow and retention](privacy.md).
- [x] Contribution, conduct, issue and PR templates, [roadmap](roadmap.md), and [support routes](../SUPPORT.md) are present.
- [ ] Confirm first-run setup with a new Windows user. Verify that missing devices, models, and provider credentials show a useful recovery path.
- [ ] Prepare a small set of verified starter issues. Link each to code or a reproducible setup problem. Set a sustainable triage cadence before inviting contributors.
- [ ] Review each platform's current rules, replace draft placeholders, and verify the final public URLs while signed out. [Launch library](launch/README.md)
