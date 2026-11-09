# OpenKeet launch kit

## Project copy

**Short.** OpenKeet is a real-time interview copilot for Windows with local transcription and live answer suggestions.

**Candidate introduction.** Practice likely questions with an interview pack based on your experience. In sessions that allow live AI help, OpenKeet follows the conversation, flags likely questions, and offers structured answers you can put in your own words.

**Developer introduction.** OpenKeet combines local speech recognition, question detection, and optional answer providers in a Windows desktop app. Help improve the setup, accessibility, transcription, or practice experience.

**Community invitation.** Try it with a practice question and tell us what would make interview prep easier for you. First-time contributors are welcome.

## Media

- **GitHub card.** Use the 1280 × 640 [social preview](../assets/social-preview.png). Alt text can read “OpenKeet. Your personal AI interview assistant. Interview smarter. Answer better.”
- **Screenshots.** Use the [Windows gallery](screenshots.md). Recapture after a UI change with the [native smoke test](../desktop-slice.md#build-and-test). Use synthetic session text and include alt text.
- **Short demo.** Connect an answer model, ask about shared state in Kotlin, then ask a follow-up about a blocking API call, and explain how to practice the answer in your own words. Add captions. Use a consented synthetic recording for any live-audio demo.
- **Feature spotlight.** Start with a problem someone preparing for an interview faces. Show one working feature, the setup it needs, and its limits.
- **Contributor thanks.** Thank contributors for a specific change in the release notes. Use a display name they have chosen to share.

## Release notes template

Use these headings when a verified package is ready. Add only claims backed by the tag and checks.

1. **What's new**. Describe the candidate-facing change.
2. **Improvements and fixes**. List problems users can now solve.
3. **Known limitations**. Name the affected workflows and available workarounds.
4. **Installation**. Link the verified Windows artifact, checksum, prerequisites, and signing status.
5. **Full changelog**. Link the GitHub comparison from the previous tag.

## Community announcements

Draft headline for Product Hunt: **OpenKeet — Your personal AI interview assistant**.

Draft headline for Show HN after the public release and license review: **Show HN: OpenKeet — an open-source personal AI interview assistant**.

Check each community's rules and publish only when the referenced download or build works for its audience. [Release readiness](../release-readiness.md)
