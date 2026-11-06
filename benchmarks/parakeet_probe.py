"""Score NVIDIA's local Parakeet model on the same files, using whole-clip decode."""
import argparse
import hashlib
from pathlib import Path
import sys
import time

from asr import RATE, read_json, read_wav, score, write_json


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True, help="Directory containing the existing WAV manifest")
    parser.add_argument("--model", type=Path, required=True, help="Verified sherpa-onnx Parakeet v3 INT8 model directory")
    parser.add_argument("--runtime-dir", type=Path, help="Optional Python package target directory")
    args = parser.parse_args()
    if args.runtime_dir:
        sys.path.insert(0, str(args.runtime_dir.resolve()))
    import sherpa_onnx

    files = [args.model / name for name in ("encoder.int8.onnx", "decoder.int8.onnx", "joiner.int8.onnx", "tokens.txt")]
    if not all(path.is_file() for path in files):
        raise ValueError("Use the extracted Parakeet v3 INT8 model directory.")
    started = time.perf_counter()
    recognizer = sherpa_onnx.OfflineRecognizer.from_transducer(
        *(str(path) for path in files), model_type="nemo_transducer", num_threads=6, provider="cpu",
    )
    load_s = time.perf_counter() - started
    rows = []
    for entry in read_json(args.input / "manifest.json"):
        path = args.input / entry["file"]
        if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
            raise ValueError(f"Fixture checksum changed: {path.name}")
        started = time.perf_counter()
        stream = recognizer.create_stream()
        stream.accept_waveform(RATE, read_wav(path))
        recognizer.decode_stream(stream)
        text = stream.result.text.strip()
        rows.append({**entry, **score(entry["reference"], text, entry["terms"]), "actual": text,
                     "offline_decode_s": time.perf_counter() - started})
    summary = {key: sum(row[key] for row in rows) for key in
               ("word_errors", "words", "char_errors", "chars", "term_hits", "terms", "missed",
                "false_transcript", "duration_s", "offline_decode_s")}
    summary.update(wer=summary["word_errors"] / max(1, summary["words"]),
                   cer=summary["char_errors"] / max(1, summary["chars"]), load_s=load_s,
                   offline_rtf=summary["offline_decode_s"] / summary["duration_s"])
    write_json(args.input / "nvidia-parakeet-v3-offline.json", {"summary": summary, "clips": rows})
    print(summary)


if __name__ == "__main__":
    main()
