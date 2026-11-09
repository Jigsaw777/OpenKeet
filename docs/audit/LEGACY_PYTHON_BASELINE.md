# Legacy Python baseline

The requested `personal_copilot(1).zip` was absent from the workspace, Downloads, Documents, Desktop, adjacent StudioProjects, the provided attachments, and the available 7z entry list. A ZIP SHA-256 and exact archive run are unavailable.

The fallback is the pre-migration Git revision `69616f4` with tree `04bfadf973cb30880fdd487cb3bbef9645416e73`. `git diff 69616f4 HEAD -- app.py audio.py live.py core.py providers.py requirements.txt` had no changes at the baseline. The paired audio runner imports the unchanged [audio.py](../../audio.py#L84) and uses the legacy `SpeechListener` without starting Tkinter. Python launchers, manual input, audio, and user data stay available.

| Area | Evidence |
| --- | --- |
| Capture | [audio.py](../../audio.py#L23) enumerates SoundCard loopback and sounddevice microphone. [SpeechSegmenter](../../audio.py#L45) uses RMS and fixed padding. |
| Model | [audio.py](../../audio.py#L84) loads Faster-Whisper on CPU with INT8. The paired run uses `small.en`. |
| Context | [core.py](../../core.py#L76) manages profiles and document import. [QuestionBuffer](../../live.py#L191) groups transcript fragments and [commit_checked_answer](../../live.py#L384) writes checked responses into memory. |
| Provider | [providers.py](../../providers.py#L63) uses user-installed CLI processes. [core.py](../../core.py#L227) connects to local Ollama. The ASR benchmark requires neither. |
| Process measurement | The former `benchmarks/profile_engines.py` (now replaced by `copilot bench-asr profile`) targeted the actual interpreter and rejected a venv-launcher measurement. It reports ASR process metrics separately from desktop process-tree metrics. |

See [performance baseline](PERFORMANCE_BASELINE.md) for the paired input, hardware, and raw output locations.
