"""Compare an optimized Rust CLI with one frozen paired Python/Windows baseline."""
import argparse
import csv
import hashlib
from pathlib import Path

from asr import ROOT, read_json, rust, summarize, write_json


def metric_rows(reports):
    indexed = [[(clip["id"], clip["sha256"]) for clip in report["clips"]] for report in reports]
    if not indexed[0] or any(index != indexed[0] for index in indexed[1:]):
        raise ValueError("Baseline and candidate must use the same ordered IDs and WAV hashes.")

    for key, label in (
        ("final_latency_s", "Final latency"),
        ("stable_partial_s", "Stable preview latency"),
    ):
        common = [triple for triple in zip(*(report["clips"] for report in reports))
                  if all(clip[key] is not None for clip in triple)]
        if common:
            stats = [summarize(rows)[key.removesuffix("_s")] for rows in zip(*common)]
            for quantile in ("p50", "p95"):
                yield f"{label} {quantile} ({len(common)} common clips)", "s", [
                    item[f"{quantile}_s"] for item in stats
                ]

    for key, label, unit in (
        ("cpu_s", "Process CPU", "s"),
        ("wall_s", "Process wall time", "s"),
        ("peak_working_set_mib", "Peak resident working set", "MiB"),
        ("median_working_set_mib", "Median resident working set", "MiB"),
        ("peak_private_commit_mib", "Peak private commit", "MiB"),
    ):
        yield label, unit, [report["process"][key] for report in reports]
    for key, label, unit, factor in (
        ("load_s", "Model load", "s", 1),
        ("wer", "Word error rate", "%", 100),
        ("cer", "Character error rate", "%", 100),
        ("term_hits", "Technical term matches", "matches", 1),
        ("missed", "Missed speech clips", "clips", 1),
        ("false_transcript", "False transcripts in controls", "clips", 1),
        ("dropped_frames", "Reported frame queue drops", "events", 1),
        ("dropped_finals", "Reported final queue drops", "events", 1),
    ):
        yield label, unit, [round(report["summary"][key] * factor, 3) for report in reports]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True, help="Output from profile_engines.py")
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--scratch", type=Path, default=ROOT / "output" / "optimized-3.0")
    parser.add_argument("--output", type=Path, default=ROOT / "results")
    args = parser.parse_args()

    baseline = args.baseline.resolve()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=True)
    manifest = baseline / "manifest.json"
    manifest_sha = hashlib.sha256(manifest.read_bytes()).hexdigest()
    summary = read_json(baseline / "comparison.json")
    if summary["manifest_sha256"] != manifest_sha:
        raise ValueError("The baseline manifest changed after its measurements.")
    entries = [{**clip, "file": str((baseline / clip["file"]).resolve())}
               for clip in read_json(manifest)]
    write_json(scratch / "manifest.json", entries)

    python = read_json(baseline / "legacy-small.en.json")
    python["process"] = summary["python"]["process"]
    before = read_json(baseline / f"rust-{args.model.stem}.json")
    after = rust(scratch, args.cli, args.model, "", profile=True)
    reports = [python, before, after]
    data = [
        [label, unit, *values, round(values[2] - values[0], 3), round(values[2] - values[1], 3)]
        for label, unit, values in metric_rows(reports)
    ]
    output = args.output
    output.mkdir(parents=True, exist_ok=True)
    for name, report in zip(("python-baseline", "current-before", "optimized-after"), reports):
        # The corpus is synthetic. Strip host paths to keep reports portable.
        clips = [{key: value for key, value in clip.items() if key != "file"} for clip in report["clips"]]
        write_json(output / f"{name}.json", {**report, "clips": clips, "manifest_sha256": manifest_sha})
    header = ["Metric", "Unit", "Python A", "Current B", "Optimized C", "C − A", "C − B"]
    with (output / "comparison.csv").open("w", encoding="utf-8", newline="") as target:
        csv.writer(target).writerows([header, *data])
    rows = [header, ["---", "---", "---:", "---:", "---:", "---:", "---:"], *data]
    table = "\n".join("| " + " | ".join(map(str, row)) + " |" for row in rows)
    (output / "comparison.md").write_text(
        "# Paired ASR process comparison\n\n"
        f"One sequential run per engine, {len(entries)} clips. Manifest SHA-256 `{manifest_sha}`. "
        "WER/CER differences are percentage points. The table excludes desktop UI and LLM costs.\n\n"
        f"{table}\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
