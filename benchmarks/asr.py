"""Generate non-personal fixtures and benchmark the current listener against them."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import statistics
import subprocess
import sys
import threading
import time
from unittest.mock import patch
import wave

import numpy as np

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent))
from audio import SPEECH_MODELS, SpeechListener

RATE = SpeechListener.RATE
VOICES = ("Microsoft David Desktop", "Microsoft Hazel Desktop", "Microsoft Zira Desktop")
STRESS_VOICES = {
    "Microsoft David": "en-US", "Microsoft Mark": "en-US", "Microsoft Zira": "en-US",
    "Microsoft George": "en-GB", "Microsoft Hazel": "en-GB", "Microsoft Susan": "en-GB",
    "Microsoft Heera": "en-IN", "Microsoft Ravi": "en-IN",
}


def write_json(path, data):
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def read_wav(path):
    with wave.open(str(path), "rb") as stream:
        if (stream.getnchannels(), stream.getsampwidth(), stream.getframerate()) != (1, 2, RATE):
            raise ValueError(f"Expected mono 16-bit {RATE} Hz PCM in {path.name}")
        return np.frombuffer(stream.readframes(stream.getnframes()), dtype="<i2").astype(np.float32) / 32768


def normalize_audio(path):
    import av

    resampler = av.AudioResampler(format="s16", layout="mono", rate=RATE)
    with av.open(str(path), mode="r") as container:
        frames = [output.to_ndarray().ravel() for frame in container.decode(audio=0)
                  for output in resampler.resample(frame)]
    frames.extend(output.to_ndarray().ravel() for output in resampler.resample(None))
    return np.concatenate(frames).astype(np.float32) / 32768


def telephone_band(samples):
    """Simulate a low-pass 8 kHz path with G.711-style companding."""
    cutoff = 3_400 / RATE
    kernel = 2 * cutoff * np.sinc(2 * cutoff * np.arange(-31, 32)) * np.hamming(63)
    narrow = np.clip(np.convolve(samples, kernel / np.sum(kernel), mode="same")[::2], -1, 1)
    encoded = np.sign(narrow) * np.log1p(255 * np.abs(narrow)) / np.log1p(255)
    quantized = np.rint(127 * encoded) / 127
    decoded = np.sign(quantized) * np.expm1(np.log1p(255) * np.abs(quantized)) / 255
    return np.interp(np.arange(len(samples)), np.arange(0, len(samples), 2), decoded).astype(np.float32)


def control_audio(control, rng):
    samples = np.zeros(RATE * 3, dtype=np.float32)
    if control == "tone":
        samples[:RATE * 2] = 0.09 * np.sin(2 * np.pi * 440 * np.arange(RATE * 2) / RATE)
    elif control == "keyboard":
        for start in np.linspace(0.2, 2.3, 13):
            offset = int(RATE * start)
            samples[offset:offset + 400] = 0.45 * rng.uniform(-1, 1, 400) * np.exp(-np.arange(400) / 60)
    elif control == "chords":
        for index, frequencies in enumerate(((220, 277, 330), (196, 247, 294), (262, 330, 392))):
            times = np.arange(RATE * 3 // 4) / RATE
            chord = sum(np.sin(2 * np.pi * hz * times) for hz in frequencies)
            start = index * len(times)
            samples[start:start + len(times)] = 0.05 * np.sin(np.pi * times / 0.75) * chord
    return samples


def generate(output, *, stress=False, corpus=None):
    output.mkdir(parents=True, exist_ok=True)
    jobs, entries = [], []
    rng = np.random.default_rng(20261106)
    voices = STRESS_VOICES if stress else VOICES
    default_corpus = ROOT / ("stress-corpus.json" if stress else "corpus.json")
    for fixture in read_json(corpus or default_corpus):
        for voice_index, voice in enumerate(voices if fixture["parts"] else (None,)):
            name = f'{fixture["id"]}-{voice_index}' if voice else fixture["id"]
            parts = [f"{name}-part{index}.wav" for index in range(len(fixture["parts"]))]
            rate = fixture.get("speaking_rate", 1) if stress else fixture.get("rate", 0)
            jobs.extend({"voice": voice, "rate": rate, "file": part, "text": phrase}
                        for part, phrase in zip(parts, fixture["parts"]))
            entries.append((fixture, name, voice, parts))

    write_json(output / "synthesis.json", jobs)
    synth_script = "synthesize-onecore.ps1" if stress else "synthesize.ps1"
    subprocess.run(["powershell.exe", "-NoProfile", "-File", str(ROOT / synth_script),
                    "-Jobs", str(output / "synthesis.json"), "-Output", str(output)], check=True)
    manifest = []
    for fixture, name, voice, parts in entries:
        if parts:
            chunks = [np.zeros(RATE // 4, dtype=np.float32)]
            for index, part in enumerate(parts):
                samples = normalize_audio(output / part) if stress else read_wav(output / part)
                signal = np.flatnonzero(np.abs(samples) > 0.002)
                start = signal[0] + RATE * fixture.get("clip_start_ms", 0) // 1000
                samples = samples[start:signal[-1] + 1]
                if index:
                    chunks.append(np.zeros(RATE * fixture["gap_ms"] // 1000, dtype=np.float32))
                chunks.append(samples)
            samples = np.concatenate(chunks)
            speech_end = len(samples) / RATE
            if fixture.get("telephone"):
                samples = telephone_band(samples)
            if "peak" in fixture:
                samples *= fixture["peak"] / np.max(np.abs(samples))
            if "snr_db" in fixture:
                noise_rms = np.sqrt(np.mean(samples ** 2)) / 10 ** (fixture["snr_db"] / 20)
                samples += rng.normal(0, noise_rms, len(samples)).astype(np.float32)
            samples = np.pad(samples, (0, RATE * 3 // 2))
        else:
            speech_end = None
            samples = control_audio(fixture["control"], rng)
        path = output / f"{name}.wav"
        with wave.open(str(path), "wb") as stream:
            stream.setparams((1, 2, RATE, 0, "NONE", "not compressed"))
            stream.writeframes(np.rint(np.clip(samples, -1, 0.9999) * 32768).astype("<i2").tobytes())
        manifest.append({"id": name, "case": fixture["id"], "voice": voice, "file": path.name,
                         "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                         "reference": " ".join(fixture["parts"]), "terms": fixture.get("terms", []),
                         "duration_s": len(samples) / RATE, "speech_end_s": speech_end})
        if stress:
            manifest[-1].update(locale=STRESS_VOICES.get(voice), condition=fixture["condition"])
    write_json(output / "manifest.json", manifest)
    print(f"Generated {len(manifest)} clips ({sum(x['duration_s'] for x in manifest):.1f} s)", flush=True)


def words(text):
    return re.findall(r"[^\W_]+(?:'[^\W_]+)?", text.casefold().replace("’", "'"))


def distance(reference, actual):
    row = list(range(len(actual) + 1))
    for index, expected in enumerate(reference, 1):
        next_row = [index]
        for column, received in enumerate(actual, 1):
            next_row.append(min(next_row[-1] + 1, row[column] + 1, row[column - 1] + (expected != received)))
        row = next_row
    return row[-1]


def score(reference, actual, terms):
    expected, received = words(reference), words(actual)
    expected_chars, received_chars = "".join(expected), "".join(received)
    phrase = f" {' '.join(received)} "
    return {"word_errors": distance(expected, received), "words": len(expected),
            "char_errors": distance(expected_chars, received_chars), "chars": len(expected_chars),
            "term_hits": sum(f" {' '.join(words(term))} " in phrase for term in terms),
            "terms": len(terms), "missed": bool(expected and not received),
            "false_transcript": bool(not expected and received)}


def stable_partial(events):
    by_utterance = {}
    for kind, at, value in events:
        if kind in ("partial_transcript", "transcript"):
            by_utterance.setdefault(value["utterance_id"], []).append((kind, at, words(value["text"])))
    candidates = []
    for stream in by_utterance.values():
        if stream[-1][0] != "transcript" or not stream[-1][2]:
            continue
        final = stream[-1][2]
        prefix = final[:2]
        for first, second in zip(stream, stream[1:]):
            if first[0] == "partial_transcript" and first[2][:len(prefix)] == second[2][:len(prefix)] == prefix:
                candidates.append(first[1])
                break
    return min(candidates, default=None)


def listen(listener, samples):
    """Replay real-time 30 ms input through the production listener and decoder."""
    import sounddevice

    events, finished = [], threading.Event()
    started = time.monotonic()

    def emit(kind, value):
        events.append((kind, time.monotonic() - started, value))
        if kind == "audio_finished":
            finished.set()

    class ReplayStream:
        def __init__(self, callback, **_):
            self.callback = callback
            self.thread = threading.Thread(target=self.feed, daemon=True)

        def __enter__(self):
            self.thread.start()
            return self

        def __exit__(self, *_):
            self.thread.join()

        def feed(self):
            nonlocal started
            started = time.monotonic()
            for offset in range(0, len(samples), listener.BLOCK):
                block = samples[offset:offset + listener.BLOCK]
                due = started + (offset + len(block)) / RATE
                if listener.stop_event.wait(max(0, due - time.monotonic())):
                    return
                self.callback(block.reshape(-1, 1), len(block), None, None)
            listener.stop()

    listener.emit = emit
    with patch.object(sounddevice, "InputStream", ReplayStream):
        listener.start(("input", None))
        if not finished.wait(len(samples) / RATE + 90):
            listener.stop()
            raise TimeoutError("Legacy transcription worker did not finish")
        for thread in listener.threads:
            thread.join()
    if listener.worker_error:
        raise RuntimeError(listener.worker_error)
    return events


def summarize(rows):
    def stats(key):
        data = sorted(row[key] for row in rows if row[key] is not None)
        if not data:
            return {"n": 0}
        return {"n": len(data), "p50_s": round(statistics.median(data), 3),
                "p95_s": round(data[math.ceil(0.95 * len(data)) - 1], 3)}

    result = {key: sum(row[key] for row in rows) for key in
              ("word_errors", "words", "char_errors", "chars", "term_hits", "terms", "missed",
               "false_transcript", "dropped_finals", "dropped_frames", "discontinuities")}
    result.update(wer=round(result["word_errors"] / max(1, result["words"]), 4),
                  cer=round(result["char_errors"] / max(1, result["chars"]), 4),
                  final_latency=stats("final_latency_s"), stable_partial=stats("stable_partial_s"),
                  reported_decode_s=round(sum(row["reported_decode_s"] for row in rows), 3),
                  audio_s=round(sum(row["duration_s"] for row in rows), 3))
    return result


def legacy(output, names):
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"
    manifest = read_json(output / "manifest.json")
    for name in names:
        listener = SpeechListener(lambda *_: None)
        loaded = time.monotonic()
        listener.load_model(name)
        load_s = time.monotonic() - loaded
        print(f"Running {name} on {len(manifest)} clips", flush=True)
        rows = []
        cpu, wall = time.process_time(), time.monotonic()
        for entry in manifest:
            path = output / entry["file"]
            if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
                raise ValueError(f"Fixture checksum changed: {path.name}")
            events = listen(listener, read_wav(path))
            finals = [(at, data) for kind, at, data in events if kind == "transcript"]
            actual = " ".join(value["text"] for _, value in finals)
            rows.append({**entry, **score(entry["reference"], actual, entry["terms"]),
                         "actual": actual, "stable_partial_s": stable_partial(events),
                         "final_latency_s": finals[-1][0] - entry["speech_end_s"]
                         if finals and entry["speech_end_s"] is not None else None,
                         "reported_decode_s": sum(data["transcription_seconds"] for kind, _, data in events
                                                  if kind in ("partial_transcript", "transcript")),
                         "dropped_finals": sum(kind == "speech_dropped" for kind, _, _ in events),
                         "dropped_frames": sum(kind == "audio_status" and "queue full" in value for kind, _, value in events),
                         "discontinuities": 0})
        summary = {**summarize(rows), "load_s": round(load_s, 3), "cpu_s": round(time.process_time() - cpu, 3),
                   "wall_s": round(time.monotonic() - wall, 3)}
        write_json(output / f"legacy-{name}.json", {"backend": name, "summary": summary, "clips": rows})
        print(f"{name} {json.dumps(summary)}", flush=True)


def rust(output, cli, model, vocabulary, *, profile=False, short_followups=False):
    manifest = read_json(output / "manifest.json")
    clips = [output / entry["file"] for entry in manifest]
    for entry, path in zip(manifest, clips):
        if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
            raise ValueError(f"Fixture checksum changed: {path.name}")
    print(f"Running Rust {model.name} on {len(clips)} clips", flush=True)
    started = time.monotonic()
    command = [str(cli.resolve()), "transcribe", str(model.resolve())]
    if vocabulary:
        command.extend(["--vocabulary", vocabulary])
    if short_followups:
        command.append("--short-followups")
    command.extend(map(str, clips))
    timeout = sum(entry["duration_s"] for entry in manifest) * 4 + 180
    metrics = None
    if profile:
        from process_metrics import measure

        process, metrics = measure(command, timeout)
    else:
        process = subprocess.run(command, capture_output=True, encoding="utf-8", timeout=timeout)
    if process.returncode:
        raise RuntimeError(f"Rust comparator exited with {process.returncode}: {process.stderr.strip()}")
    events, rows = [], []
    speech_segments = 0
    load_s = None
    for line in process.stdout.splitlines():
        item = json.loads(line)
        if item["event"] == "model_ready":
            load_s = item["load_ms"] / 1000
        elif item["event"] == "clip_start":
            events = []
            speech_segments = 0
        elif item["event"] == "speech":
            speech_segments += 1
        elif item["event"] == "transcript":
            utterance = item["utterance"]
            kind = "transcript" if utterance["stability"] == "final" else "partial_transcript"
            events.append((kind, item["at_ms"] / 1000,
                           {"text": utterance["text"], "utterance_id": utterance["id"],
                            "transcription_seconds": item["decode_ms"] / 1000,
                            "queue_seconds": item["queue_ms"] / 1000}))
        elif item["event"] == "clip_end":
            entry = manifest[item["index"]]
            finals = [(at, data) for kind, at, data in events if kind == "transcript" and data["text"]]
            actual = " ".join(value["text"] for _, value in finals)
            rows.append({**entry, **score(entry["reference"], actual, entry["terms"]), "actual": actual,
                         "speech_segments": speech_segments,
                         "empty_final_segments": sum(kind == "transcript" and not data["text"] for kind, _, data in events),
                         "stable_partial_s": stable_partial(events),
                         "final_latency_s": finals[-1][0] - entry["speech_end_s"]
                         if finals and entry["speech_end_s"] is not None else None,
                         "reported_decode_s": sum(data["transcription_seconds"] for _, _, data in events if data["text"]),
                         "max_decode_queue_s": max((data["queue_seconds"] for _, _, data in events), default=0),
                         "dropped_finals": 0, "dropped_frames": 0, "discontinuities": 0})
    if [row["id"] for row in rows] != [entry["id"] for entry in manifest]:
        raise RuntimeError("Rust comparator did not finish every fixture in manifest order")
    summary = {**summarize(rows), "load_s": load_s, "wall_s_with_load": round(time.monotonic() - started, 3),
               "max_decode_queue_s": max(row["max_decode_queue_s"] for row in rows)}
    label = f"rust-{model.stem}{'-hints' if vocabulary else ''}{'-short-followups' if short_followups else ''}"
    report = {"backend": label, "vocabulary": vocabulary, "summary": summary, "clips": rows}
    if metrics:
        report["process"] = metrics
    write_json(output / f"{label}.json", report)
    print(f"{label} {json.dumps(summary)}", flush=True)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    commands = parser.add_subparsers(dest="command", required=True)
    generator = commands.add_parser("generate")
    generator.add_argument("--stress", action="store_true")
    generator.add_argument("--corpus", type=Path)
    commands.add_parser("legacy").add_argument("--models", choices=SPEECH_MODELS, nargs="+", default=SPEECH_MODELS)
    candidate = commands.add_parser("rust")
    candidate.add_argument("--cli", type=Path, required=True)
    candidate.add_argument("--model", type=Path, required=True)
    candidate.add_argument("--vocabulary", default="")
    candidate.add_argument("--short-followups", action="store_true")
    args = parser.parse_args()
    stress = getattr(args, "stress", False)
    output = (args.output or ROOT / "output" / ("stress" if stress else "")).resolve()
    if args.command == "generate":
        generate(output, stress=stress, corpus=args.corpus)
    elif args.command == "legacy":
        legacy(output, args.models)
    else:
        rust(output, args.cli, args.model, args.vocabulary, short_followups=args.short_followups)


if __name__ == "__main__":
    main()
