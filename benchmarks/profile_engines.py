"""Profile the shipped Python and Rust ASR pipelines on a fixed paired stress subset."""
import argparse
import hashlib
import os
from pathlib import Path
import sys

from asr import ROOT, read_json, rust, summarize, write_json
from process_metrics import measure

VOICES = {"Microsoft Mark", "Microsoft Hazel", "Microsoft Heera"}
CASES = {"seashells", "seashells-fast", "sixth-quiet", "cash-cache-noise", "tech-telephone", "why-fast", "correction-pause"}


def profile_legacy(output, timeout):
    # Windows venv's python.exe can be a launcher. Profile the actual interpreter with the same venv.
    env = {**os.environ, "__PYVENV_LAUNCHER__": sys.executable}
    legacy, metrics = measure([sys._base_executable, "-X", "utf8", str(ROOT / "asr.py"), "--output", str(output),
                               "legacy", "--models", "small.en"], timeout, env=env)
    print(legacy.stdout, end="", flush=True)
    if legacy.returncode:
        raise RuntimeError(legacy.stderr.strip() or "Python baseline exited before finishing")
    if metrics["cpu_s"] <= 0 or metrics["peak_working_set_mib"] < 64:
        raise RuntimeError("The profiler appears to have attached to a launcher instead of the ASR interpreter.")
    return {"process": metrics, "asr": read_json(output / "legacy-small.en.json")["summary"]}


def matched_latencies(python_rows, rust_rows):
    if [(row["id"], row["sha256"]) for row in python_rows] != [(row["id"], row["sha256"]) for row in rust_rows]:
        raise ValueError("Profile results do not contain the same input clips and hashes.")
    result = {}
    for key in ("final_latency", "stable_partial"):
        metric = f"{key}_s"
        common = [(a, b) for a, b in zip(python_rows, rust_rows) if a[metric] is not None and b[metric] is not None]
        result[key] = {"python": summarize([a for a, _ in common])[key], "rust": summarize([b for _, b in common])[key]}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stress", type=Path, default=ROOT / "output" / "stress")
    parser.add_argument("--output", type=Path, default=ROOT / "output" / "profile")
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--model", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    stress = args.stress.resolve()
    output.mkdir(parents=True, exist_ok=True)
    rows = []
    for entry in read_json(stress / "manifest.json"):
        if entry["condition"] == "control" or (entry["voice"] in VOICES and entry["case"] in CASES):
            rows.append({**entry, "file": str(stress / entry["file"])})
    if len(rows) != len(VOICES) * len(CASES) + 4:
        raise ValueError("Generate the complete Windows stress pack before profiling.")
    manifest = output / "manifest.json"
    write_json(manifest, rows)
    print(f"Profiling both engines on {len(rows)} identical clips", flush=True)
    timeout = sum(entry["duration_s"] for entry in rows) * 5 + 120
    legacy = profile_legacy(output, timeout)
    rust_report = rust(output, args.cli, args.model, "", profile=True)
    comparison = {"manifest_sha256": hashlib.sha256(manifest.read_bytes()).hexdigest(), "clips": len(rows),
                  "python": legacy,
                  "rust": {"process": rust_report["process"], "asr": rust_report["summary"]},
                  "matched_latency": matched_latencies(read_json(output / "legacy-small.en.json")["clips"], rust_report["clips"])}
    write_json(output / "comparison.json", comparison)
    for name in ("python", "rust"):
        print(f"{name} process {comparison[name]['process']}", flush=True)


if __name__ == "__main__":
    main()
