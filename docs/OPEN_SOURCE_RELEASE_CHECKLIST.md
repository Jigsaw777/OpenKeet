# OpenKeet public release checklist

**Decision on 8 November 2026: Not ready for public release.** The repository is private and the Windows source preview can be built. A public launch needs the evidence below and the owner's explicit approval.

| Severity | Gate | Evidence required |
| --- | --- | --- |
| Blocker | Source and asset redistribution | Sign off on original and adapted code, logo artwork, and the outlined font. Preserve required notices. The repository declares MIT for project source. [Dependency and license audit](audit/DEPENDENCY_AND_LICENSE_AUDIT.md) |
| Blocker | Verified public installer | Build the final Windows 11 x64 package from its tag. Verify installation and uninstall on a clean account, signing and checksum, a software bill of materials, and third-party notices. [Release readiness](release-readiness.md) |
| Blocker | Private vulnerability contact | Set up and verify a direct private channel before launch. The current policy tells reporters how to request private contact. [Security policy](../SECURITY.md) |
| Blocker | Security and privacy review of final refs and artifacts | Repeat a redacted history scan. Review fixtures, screenshots, build logs, package contents, and endpoint configuration. Record how any finding was resolved before exposure. |
| High | Provider and device evidence | Verify real provider authorization, limits and disconnection. Recheck physical mic and headset switching and capture permissions. Keep unverified provider behavior marked as preview. [Provider audit](audit/PROVIDER_AUTHORIZATION.md) |
| High | Maintainer sign-off | Review the exact repository, release and community drafts. Explicit approval is required for public visibility and announcements. |

## Security

- [x] Gitleaks found no matches across 37 local history and reflog commits in the rename preflight. The scan used full redaction. Pattern matching is one input to the final review.
- [x] GitHub reported zero repository Actions secrets and variables. PR runs use a read-only token. Private fork workflow execution, secret sharing, and write tokens were disabled.
- [ ] Review the final published refs, fixture data, generated logs, screenshots, and installer for personal information, debug credentials, keys, and private endpoints. Test files should contain synthetic or licensed inputs.
- [ ] Document the installer source tag, toolchain, checksum, signing, and provenance. Run dependency advisory checks against the exact lockfiles and record which findings affect the Windows package.
- [ ] Test a private report end to end with the maintainer. Use [SECURITY.md](../SECURITY.md) for the contact route.

## Licensing and intellectual property

- [x] The root [MIT license](../LICENSE) is the current project source license. The rename keeps that declaration.
- [ ] Confirm permission for inherited and adapted code. The [existing audit](audit/DEPENDENCY_AND_LICENSE_AUDIT.md) records the restricted Natively reference as research. Review legacy bridge provenance separately.
- [ ] Inventory packaged libraries, native speech runtime DLLs, installer components, and build tooling. Include their required license notices with the release.
- [ ] Review terms for model downloads, including default Whisper and experimental NVIDIA Parakeet. Keep their provenance, checksums, notices, and attribution documented. The preview downloads weights separately.
- [ ] Review supplied logo rights, wordmark font obligations, screenshots, and demo footage. Record the name and asset clearance decision. The owner namespace accepted the OpenKeet name. [Brand guide](brand/BRAND.md)
- [ ] Confirm that contributions can be distributed under the repository's declared terms. Document any exception for third-party code, data, or assets. Review any license change separately.

## Product and community

- [x] OpenKeet repository name, topic metadata, candidate-focused README, and support links have a canonical destination. [Migration audit](launch/repository-migration.md)
- [x] The docs cover Windows source installation, local transcription, answer-provider setup and billing, and [data flow and retention](privacy.md).
- [x] Contribution, conduct, issue and PR templates, [roadmap](roadmap.md), and [support routes](../SUPPORT.md) are present.
- [ ] Confirm first-run setup with a new Windows user. Verify that missing devices, models, and provider credentials show a useful recovery path.
- [ ] Prepare a small set of verified starter issues. Link each to code or a reproducible setup problem. Set a sustainable triage cadence before inviting contributors.
- [ ] Review each platform's current rules, replace draft placeholders, and verify the final public URLs while signed out. [Launch library](launch/README.md)
