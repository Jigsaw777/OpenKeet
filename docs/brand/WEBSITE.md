# OpenKeet website copy and direction

There is no website in this repository yet. This is the ready-to-build homepage. Every claim matches the current Windows desktop release. Planned features stay off the page until they ship. Tokens and type come from the [design system](DESIGN_SYSTEM.md). Use Manrope for headings, Inter for body and JetBrains Mono for code, all self-hosted with their OFL licences.

**Layout system.** Content max 72rem. A 12-column grid with 24 px gutters on desktop, 8 columns on tablet and 4 on phones with 16 px side gutters. Sections are separated by 96 px on desktop and 64 px on phones. Dark theme by default, with a light theme from `prefers-color-scheme`. Screenshots sit in a 12 px radius frame with a 1 px `#2A3552` border, never a device mock-up.

## 1. Header

Logo on dark (`openkeet-logo-on-dark.svg`, 28 px tall). Links: **How it works** · **Features** · **Privacy** · **FAQ** · **GitHub**. Button: **Download OpenKeet**.

Phones: logo, Download, and a menu button that opens the same links in a full-width sheet.

## 2. Hero

> # Interview smarter. Answer better.
>
> From technical questions to behavioral interviews, OpenKeet helps you prepare, practice, and approach challenging conversations with confidence.
>
> **[Download OpenKeet]** · **[See how it works]**
>
> Windows 11 · Speech recognition runs on your PC

*Visual.* A real screenshot of the overlay over a blurred, generic video call. It shows the question block ("What if the API call blocks?") and a checked suggested answer with a lime rule. No people's faces are needed. Text sits left and the screenshot right on desktop; on phones they stack, text first.

Say "Free download" only when the release being linked is free to use.

## 3. Product demonstration

> ## See a question turn into a clear answer
>
> Start listening, and OpenKeet follows the conversation. When the interviewer asks something, the question settles into place and a suggested answer appears beside it, ready when you are.

*Visual.* A 20–30 second silent loop recorded from the built-in sample session: Listening → Question heard → Writing answer → Answer ready. Captions name each state. Provide a poster frame and a reduced-motion still.

## 4. How OpenKeet works

> ## Three steps, then you're listening

1. **Pick what to hear.** Choose the speakers or headphones your call plays through, or a microphone.
2. **Get a speech model.** Download Whisper or NVIDIA Parakeet once. Transcription runs on your PC.
3. **Connect answers, if you want them.** Use your ChatGPT account, a Claude Console key or a local Ollama model. Without one you still get a live transcript.

**[Try the sample session]**. It plays an offline practice interview with example answers.

*Layout.* Three numbered columns on desktop, stacked on phones. Use the app's checklist styling with lime ticks.

## 5. Key features

> ## Built for the moment a question lands

| Feature | Copy |
| --- | --- |
| Live question understanding | OpenKeet transcribes as they speak and shows the question taking shape, then settling, so you're answering the right thing. |
| Suggested answers you can follow | Answers stream in with clear steps, bullet points and code blocks you can scan in seconds. |
| A window that stays out of the way | A compact see-through overlay sits over your call. Adjust its opacity, hide it, and bring it back with Ctrl+Alt+I. |
| Follow-ups stay connected | When they ask "Why?" or "What if it fails?", OpenKeet links it to the question before. |
| Type a question anytime | Ask about anything that wasn't detected, or turn the full transcript into a question. |
| Sessions you can review | Save a session, search transcripts, add notes, and export to Markdown, JSON or text. |

*Layout.* Two-by-three cards with an 18 px stroke icon, title and one line. Teal icons, with lime reserved for the "suggested answers" card.

## 6. Technical interviews

> ## Prepare for technical rounds with answers that read like code
>
> Add the terms you expect, such as Kotlin, WASAPI or PostgreSQL, so speech recognition catches them. Suggested answers keep code in readable blocks and steps in order.

*Visual.* Screenshot of an answer with a numbered list and a code block.

## 7. Behavioral interviews

> ## Find the thread of your story
>
> Behavioral questions are about your experience. OpenKeet helps you notice exactly what was asked and suggests a structure you can fill with your own examples.

Keep this section modest. There is no STAR builder or resume context in the desktop release yet.

## 8. Real-time assistance

> ## Clear status, every second
>
> Listening, Hearing speech, Question heard, Writing answer, Answer ready. You always know what OpenKeet is doing, and Start, Pause and Stop are one click away.

*Visual.* The five status pills in a row, each with its shape and label.

## 9. Local AI and privacy

> ## Choose how OpenKeet runs on your machine
>
> Speech recognition runs on your PC and audio isn't uploaded for transcription. Answers go only to the provider you pick. Choose Ollama and they stay on your machine too. Saved sessions live in a local library that you can export or delete.

Bullets:

- Local speech models: Whisper `small.en` or NVIDIA Parakeet v3, verified by checksum on download.
- Your provider, your account: ChatGPT sign-in, Claude Console key stored with Windows encryption, or local Ollama.
- You're in control: listening is always visible, and OpenKeet asks you to capture only audio you have permission to record.

## 10. Model integrations

> ## Bring the model you trust

Three tiles, **ChatGPT**, **Claude** and **Ollama (local)**, each with one line of setup and the text "Runs on: provider" or "Runs on: this PC". Use text names only. Do not use third-party logos without each company's permission.

## 11. Screenshots

A carousel of four real captures: start screen, live split view, overlay, History. Each has a one-line caption. Add arrows and dots for keyboard and touch, and no autoplay.

## 12. FAQ

**What is OpenKeet?** A Windows desktop interview copilot. It transcribes the conversation on your PC, spots questions and suggests answers from the provider you choose.

**Does it upload my audio?** No. Speech recognition runs locally. When you request an answer or turn on automatic answers, the question text and the session's earlier questions and answers go to your answer provider. With Ollama they stay on your PC.

**Which answer providers work?** ChatGPT through your account, Claude through a Claude Console API key, and local models through Ollama.

**Do I need a powerful PC?** It is tested on an Intel Core i5 laptop with 16 GB of RAM and no dedicated GPU. Speech models run on the CPU.

**Is it free?** State the actual price and licence of the release you link. The source is MIT licensed.

**Is OpenKeet hidden from screen sharing or the interviewer?** No. OpenKeet does not claim to be invisible to screen sharing, recording or proctoring tools. Use it where it is allowed, and only capture audio you have permission to record.

**Will it answer for me?** It suggests. You decide what to say. Suggested answers can be wrong, and your own experience is what interviewers are looking for.

**Which platforms?** Windows 11 x64 today.

## 13. Download

> ## Ready when you are
>
> Download OpenKeet for Windows, then try the sample session before your next interview.
>
> **[Download for Windows]** · Checksum and release notes on GitHub

## 14. Footer

Logo, the tagline, links (GitHub, Documentation, Privacy, Licence) and "© OpenKeet contributors. MIT licensed." Light text on `#080C18`.

## Responsive and accessibility rules

- Headline 3.5rem on desktop, 2.25rem on phones. Body 1.0625rem with a line height of 1.6 and a measure of 65 characters.
- Buttons are at least 44 px tall on touch, with visible focus rings in `--focus`.
- Every screenshot has alt text that says what it shows. Videos have captions and a pause control.
- Respect `prefers-reduced-motion`: no parallax, and loops become stills.
