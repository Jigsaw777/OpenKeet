# LinkedIn launch drafts

Use one at a time. Review the public links, actual build and media before posting. LinkedIn supports text and native media. If the composer doesn't support the chosen media and link together, use the text link post or add a clearly labeled release link in a follow-up comment. [LinkedIn posting guide](https://www.linkedin.com/help/linkedin/answer/a518996) · [Video help](https://www.linkedin.com/help/linkedin/answer/a7494039)

## 1. Founder announcement

**Stage and media.** First public release. Use the OpenKeet card or the verified Windows setup screen.

**Post**

Interviews are stressful enough. Preparing for them shouldn't be.

I built OpenKeet to make it easier to start with one question and work through an answer you can explain in your own words.

It's a Windows desktop app. Type a question with an answer model connected, or choose a microphone or playback device for a practice call where everyone agrees to transcription. The speech model runs on your PC. For answer suggestions, you can choose a local Ollama model or connect a supported provider.

OpenKeet is your personal AI interview assistant.

Interview smarter. Answer better.

Try the Windows release: [VERIFIED_RELEASE_DOWNLOAD_URL]

If you try it, tell me what would make the first session easier.

#OpenSource #InterviewPreparation

**Follow-up comment.** “What would you try first: a typed technical question or an audio practice session? I’m interested in the first step that feels unclear.”

## 2. Why the source is open

**Stage and media.** After source publication and license sign-off. Use a screenshot of the public repo and contribution guide.

**Post**

I want OpenKeet users to be able to see where their interview data goes.

OpenKeet transcribes audio locally. With a cloud answer provider selected, the app sends the question and relevant text for each answer request. You can use transcription only, or set up Ollama to run answers on the same PC.

Publishing the source under MIT makes those boundaries easier to inspect. It also lets someone improve a Windows setup step, a keyboard interaction, or a speech test without having to build a new app.

Built openly. Improved together.

Source and docs: [VERIFIED_GITHUB_REPOSITORY_URL]

If a setup or privacy explanation is unclear, point me to the sentence. That is a useful contribution.

#OpenSource #LocalAI

**Follow-up comment.** “The privacy guide explains which text leaves the device and when a session is saved. I’d welcome feedback from someone reading it for the first time.”

## 3. Show one real practice workflow

**Stage and media.** After a captioned walkthrough is ready. Attach a clip of a typed question and follow-up, using synthetic text.

**Post**

Here is one way to practice a technical interview answer.

I typed “When would you use an LRU cache?” into OpenKeet. I answered aloud first, then read the suggestion. The next question was “What would change if the data became stale?”

That second question is the useful part. It forces you to explain a trade-off, and it shows whether your first answer skipped an assumption.

OpenKeet can also transcribe permitted practice audio with a local speech model. In the clip, I'm using typed questions and a connected model. That path starts without audio setup.

Watch the workflow: [OFFICIAL_DEMO_VIDEO_URL]

Which kind of follow-up do you find hardest to practice?

#InterviewPreparation #SoftwareEngineering

**Follow-up comment.** “Try the same sequence with a behavioral answer. After the first answer, ask what you would do differently and why.”

## 4. Invite one useful contribution

**Stage and media.** After the maintainer has reviewed starter issues. Use an actual setup or keyboard-navigation screenshot.

**Post**

You don't need to know the whole OpenKeet codebase to help improve it.

I'm looking for focused contributions to the Windows interview practice app. Try the first-run guide on a clean setup and flag a missing step. Check how the Settings screen works with the keyboard. If you work on speech or Rust, bring one short synthetic fixture that reproduces a missed question or a recovery problem.

The app has a Rust session engine, Tauri desktop shell, and local speech options. An improvement can start with one clear report and a result we can verify.

Contribution guide: [VERIFIED_CONTRIBUTING_URL]

What would make your first change easier to start?

#OpenSource #Rust

**Follow-up comment.** “A useful report can be three lines: the step, what happened, and what you expected. Please use synthetic questions and keep credentials out of screenshots.”

## 5. Build in public with a measured trade-off

**Stage and media.** After the reproducible report is public. Use a labeled chart or table from the actual benchmark, including the worse condition.

**Post**

One speech model made fewer errors overall and still did worse on a case I cared about.

I ran Whisper small.en and experimental NVIDIA Parakeet v3 through the same paced OpenKeet pipeline on one Windows PC. On 116 synthetic clips, including four controls, the measured word error rates were 18.02% and 13.33%, respectively. On the simulated telephone-band technical-word subset, the result went the other way: 26.79% for Whisper and 40.18% for Parakeet.

That is why the default stays Whisper and Parakeet is an option. The synthetic run gives a useful comparison. Real microphones, speakers, and meeting codecs still need their own tests.

Method and results: [VERIFIED_BENCHMARK_URL]

If you work on speech systems, what would you put in the next small test set?

#SpeechRecognition #LocalAI

**Follow-up comment.** “The runs used the same segmentation and clips. I’d like to add a reproducible, consented example where a technical word or short follow-up changes the question.”
