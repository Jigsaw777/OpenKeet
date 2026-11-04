"""Local system-loopback/input capture, utterance detection and Whisper transcription."""
from __future__ import annotations

from collections import deque
import os
import queue
import sys
import threading
import time
from typing import Callable

from core import CopilotError
from live import AudioChunk, TranscriptionMailbox

SPEECH_MODELS = ("tiny.en", "base.en", "small.en")


def input_devices() -> list[tuple[int, str]]:
    try:
        import sounddevice as sd
        return [(i, d["name"]) for i, d in enumerate(sd.query_devices()) if d["max_input_channels"] > 0]
    except ImportError as exc:
        raise CopilotError("Install audio packages: python -m pip install -r requirements.txt") from exc
    except Exception as exc:
        raise CopilotError("Audio inputs unavailable. Check permissions and PortAudio.") from exc


def loopback_devices() -> list[tuple[str, str]]:
    if sys.platform == "darwin":
        return []
    try:
        import soundcard as sc
        return [(str(d.id), d.name) for d in sc.all_microphones(include_loopback=True) if d.isloopback]
    except ImportError as exc:
        raise CopilotError("Install SoundCard for Windows/Linux system audio.") from exc
    except Exception as exc:
        raise CopilotError("System output enumeration failed. Check WASAPI or PulseAudio/PipeWire.") from exc


class SpeechSegmenter:
    RATE = 16000
    BLOCK = 480

    def __init__(self, threshold=0.012, silence_seconds=0.45):
        self.threshold = threshold
        self.silence_blocks = max(5, round(silence_seconds * self.RATE / self.BLOCK))
        self.pre_roll = deque(maxlen=10)
        self.parts = []
        self.silent = self.voiced = 0
        self.last_voice_at = None

    def feed(self, samples, captured_at):
        import numpy as np
        active = float(np.sqrt(np.mean(samples * samples))) > self.threshold
        if active:
            self.last_voice_at = captured_at
        if not self.parts:
            self.pre_roll.append(samples)
            if active:
                self.parts = list(self.pre_roll)
                self.pre_roll.clear()
                self.voiced, self.silent = 1, 0
        else:
            self.parts.append(samples)
            self.silent = 0 if active else self.silent + 1
            self.voiced += int(active)
            if self.silent >= self.silence_blocks or len(self.parts) >= 800:
                return self.flush()
        return None

    def flush(self):
        import numpy as np
        result = (np.concatenate(self.parts), self.last_voice_at) if self.parts and self.voiced >= 8 else None
        self.parts, self.voiced, self.silent = [], 0, 0
        self.pre_roll.clear()
        return result


class SpeechListener:
    RATE, BLOCK = SpeechSegmenter.RATE, SpeechSegmenter.BLOCK

    def __init__(self, emit: Callable[[str, object], None]):
        self.emit = emit
        self.model = None
        self.model_name = None
        self.stop_event = threading.Event()
        self.capture_done = threading.Event()
        self.frames = queue.Queue(maxsize=400)
        self.utterances = TranscriptionMailbox()
        self.threads = []
        self.worker_error = None

    def load_model(self, name="tiny.en"):
        if name not in SPEECH_MODELS:
            raise CopilotError("Select tiny.en, base.en or small.en.")
        if any(t.is_alive() for t in self.threads):
            raise CopilotError("Stop listening and wait for transcription to finish before loading another model.")
        try:
            os.environ.setdefault("HF_HUB_DISABLE_TELEMETRY", "1")
            from faster_whisper import WhisperModel
            self.model = WhisperModel(name, device="cpu", compute_type="int8")
            self.model_name = name
        except ImportError as exc:
            raise CopilotError("Install requirements.txt for local speech transcription.") from exc
        except Exception as exc:
            raise CopilotError("Speech model could not load. First use needs internet and model-cache space.") from exc

    def start(self, route: tuple[str, object], threshold=0.012, silence_seconds=0.45,
              partial_interval=0.9):
        if self.model is None:
            raise CopilotError("Load the speech model first (one download, then local inference).")
        if any(t.is_alive() for t in self.threads):
            raise CopilotError("Previous transcription is finishing. Wait a moment before restarting.")
        backend, device = route
        if backend not in ("system", "input"):
            raise CopilotError("Choose a system output or microphone/virtual input.")
        if backend == "system" and sys.platform == "darwin":
            raise CopilotError("On macOS, route output through BlackHole and choose BlackHole as the input. See README.md.")
        if backend == "system":
            # SoundCard initializes COM on first import. Import before the capture worker initializes its own apartment.
            try:
                import soundcard as sc
            except ImportError as exc:
                raise CopilotError("Install SoundCard for Windows/Linux system audio.") from exc
        self.stop_event.clear()
        self.capture_done.clear()
        self.worker_error = None
        self.frames = queue.Queue(maxsize=400)
        self.utterances = TranscriptionMailbox()
        segmenter = SpeechSegmenter(threshold, silence_seconds)

        def put_frame(samples):
            try:
                self.frames.put_nowait((samples, time.monotonic()))
            except queue.Full:
                self.emit("audio_status", "Capture queue full; a short section was skipped.")

        def capture():
            com_initialized = False
            try:
                if backend == "system":
                    if sys.platform == "win32":
                        import ctypes
                        com_initialized = ctypes.windll.ole32.CoInitializeEx(None, 0) in (0, 1)
                    import numpy as np
                    if device is None:
                        speaker = sc.default_speaker()
                        if speaker is None:
                            raise CopilotError("No default system output found.")
                    endpoint = str(speaker.id) if device is None else device
                    mic = sc.get_microphone(id=endpoint, include_loopback=True)
                    if not mic.isloopback or str(mic.id) != endpoint:
                        raise CopilotError("The selected output is unavailable. Refresh inputs and select an output.")
                    # Capture all native channels: SoundCard documents a Windows mono-capture bug.
                    # The backend resamples to 16 kHz; downmix after capture.
                    with mic.recorder(samplerate=self.RATE, blocksize=1920) as recorder:
                        self.emit("audio_status", "Listening to system output: " + mic.name)
                        next_check = time.monotonic() + 1
                        while not self.stop_event.is_set():
                            samples = recorder.record(numframes=self.BLOCK)
                            put_frame(np.asarray(samples.mean(axis=1), dtype=np.float32))
                            if sys.platform == "win32" and time.monotonic() >= next_check:
                                next_check = time.monotonic() + 1
                                if device is None and str(getattr(sc.default_speaker(), "id", "")) != endpoint:
                                    raise CopilotError("Windows changed the default output. Start listening again to use it.")
                                if not any(str(s.id) == endpoint for s in sc.all_speakers()):
                                    raise CopilotError("The selected output disconnected. Refresh inputs and select an output.")
                else:
                    import sounddevice as sd
                    def callback(data, frames, time_info, status):
                        if status:
                            self.emit("audio_status", "Input overflow: a short section may be missing.")
                        put_frame(data[:, 0].copy())
                    with sd.InputStream(samplerate=self.RATE, blocksize=self.BLOCK, device=device,
                                        channels=1, dtype="float32", callback=callback):
                        self.emit("audio_status", "Listening to the selected microphone/virtual input.")
                        while not self.stop_event.wait(0.1):
                            pass
            except CopilotError as exc:
                self.worker_error = str(exc)
                self.stop_event.set()
            except Exception:
                self.worker_error = "Capture failed. Check that the selected device is connected, refresh inputs, and restart listening."
                self.stop_event.set()
            finally:
                if com_initialized:
                    import ctypes
                    ctypes.windll.ole32.CoUninitialize()
                self.capture_done.set()

        def submit(value, utterance_id, final=True):
            if value is None:
                return
            samples, speech_at = value
            evicted = self.utterances.put(AudioChunk(utterance_id, samples, speech_at, final))
            if evicted is not None:
                self.emit("speech_dropped", evicted)

        def segment():
            import numpy as np
            utterance_id = 0
            last_partial_size = 0
            last_activity_event = float("-inf")
            partial_blocks = max(10, round(partial_interval * self.RATE / self.BLOCK))
            try:
                while not self.capture_done.is_set() or not self.frames.empty():
                    try:
                        samples, stamp = self.frames.get(timeout=0.1)
                    except queue.Empty:
                        continue
                    had_utterance = bool(segmenter.parts)
                    complete = segmenter.feed(samples, stamp)
                    if segmenter.last_voice_at == stamp and stamp - last_activity_event >= 0.12:
                        self.emit("speech_activity", {"utterance_id": utterance_id, "at": stamp})
                        last_activity_event = stamp
                    if had_utterance and not segmenter.parts:
                        if complete is not None:
                            submit(complete, utterance_id)
                        else:
                            self.emit("speech_empty", {"utterance_id": utterance_id,
                                                       "speech_end": segmenter.last_voice_at or stamp})
                        utterance_id += 1
                        last_partial_size = 0
                    elif (segmenter.parts and segmenter.voiced >= 8 and segmenter.silent == 0
                          and len(segmenter.parts) - last_partial_size >= partial_blocks):
                        # Re-decode the cumulative utterance while capture continues.
                        # The mailbox coalesces snapshots if recognition is slower than speech.
                        submit((np.concatenate(segmenter.parts), segmenter.last_voice_at),
                               utterance_id, final=False)
                        last_partial_size = len(segmenter.parts)
                submit(segmenter.flush(), utterance_id)
            except Exception:
                self.worker_error = "Audio segmentation failed. Stop listening, then retry."
                self.stop_event.set()
            finally:
                self.utterances.close()

        def transcribe():
            try:
                while True:
                    value = self.utterances.get()
                    if value is None:
                        break
                    started = time.monotonic()
                    segments, _ = self.model.transcribe(value.samples, language="en", beam_size=1,
                                                        condition_on_previous_text=False)
                    text = " ".join(item.text.strip() for item in segments).strip()
                    if text:
                        self.emit("transcript" if value.final else "partial_transcript",
                                  {"text": text, "speech_end": value.speech_at,
                                   "utterance_id": value.utterance_id,
                                                 "transcription_seconds": time.monotonic() - started})
                    elif value.final:
                        self.emit("speech_empty", {"utterance_id": value.utterance_id,
                                                   "speech_end": value.speech_at})
            except Exception:
                self.worker_error = "Local transcription failed. Stop listening and reload the speech model."
                self.stop_event.set()
                self.utterances.close()
            finally:
                if self.worker_error:
                    self.emit("audio_error", self.worker_error)
                self.emit("audio_finished", None)

        self.threads = [threading.Thread(target=fn, daemon=True) for fn in (capture, segment, transcribe)]
        for thread in reversed(self.threads):
            thread.start()

    def stop(self):
        # Streams close in the capture thread, avoiding cross-thread backend calls.
        self.stop_event.set()
