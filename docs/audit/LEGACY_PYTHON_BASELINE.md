# Legacy Python baseline

The requested `personal_copilot(1).zip` was absent from the workspace, Downloads, Documents, Desktop, adjacent StudioProjects, the provided attachments, and the available 7z entry list. A ZIP SHA-256 and exact archive run are unavailable.

The fallback is the pre-migration Git revision `69616f4` with tree `04bfadf973cb30880fdd487cb3bbef9645416e73`. At the baseline, the compared Python files matched that revision. The paired audio runner imports the archived [audio.py](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/audio.py#L84) and uses the legacy `SpeechListener` without starting Tkinter. [Migration guide](../legacy-python.md)

| Area | Evidence |
| --- | --- |
| Capture | [audio.py](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/audio.py#L23) enumerates SoundCard loopback and sounddevice microphone. [SpeechSegmenter](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/audio.py#L45) uses RMS and fixed padding. |
| Model | [audio.py](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/audio.py#L84) loads Faster-Whisper on CPU with INT8. The paired run uses `small.en`. |
| Context | [core.py](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/core.py#L76) manages profiles and document import. [QuestionBuffer](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/live.py#L191) groups transcript fragments and [commit_checked_answer](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/live.py#L384) writes checked responses into memory. |
| Provider | [providers.py](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/providers.py#L63) uses user-installed CLI processes. [core.py](https://github.com/Jigsaw777/OpenKeet/blob/769f9023f888ef8b9a19270f5b9fcf3758d1a8bd/core.py#L227) connects to local Ollama. The ASR benchmark requires neither. |
| Process measurement | The former `benchmarks/profile_engines.py` (now replaced by `copilot bench-asr profile`) targeted the actual interpreter and rejected a venv-launcher measurement. It reports ASR process metrics separately from desktop process-tree metrics. |

See [performance baseline](PERFORMANCE_BASELINE.md) for the paired input, hardware, and raw output locations.
