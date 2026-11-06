# Reproduce the Personal Copilot 3.0 ASR comparison

Run these on Windows with the same synthetic WAVs and the legacy Python environment. The setup in [audio benchmarks](../docs/benchmarks/audio.md) covers the model and Python dependencies. [Baseline provenance](../docs/audit/PERFORMANCE_BASELINE.md) identifies the source and hardware. Exact archive A was unavailable and the table uses the unchanged Git-era Python ASR.

1. Keep the original 116-clip synthetic pack or generate it with `.\.venv\Scripts\python.exe -B benchmarks/asr.py generate --stress`. Source text is in [stress-corpus.json](stress-corpus.json). The generated WAVs and their hashes live under `benchmarks/output/stress`.
2. Build a release audio CLI from archive B source `d2d8069` with `cargo build --locked --release -p copilot --features audio`. Keep that executable before building C. Set `LIBCLANG_PATH` to the directory containing `libclang.dll` when building whisper-rs on Windows.
3. From this checkout run `.\.venv\Scripts\python.exe -B benchmarks/profile_engines.py --cli <baseline-cli> --model models/ggml-small.en.bin --output benchmarks/output/baseline-3.0/profile`. This executes A and B sequentially.
4. Build the release CLI from the candidate and run `.\.venv\Scripts\python.exe -B benchmarks/run_comparison.py --baseline benchmarks/output/baseline-3.0/profile --cli target/release/copilot.exe --model models/ggml-small.en.bin`. The script verifies manifest and WAV hashes and writes the table plus synthetic per-clip results to [results](results). Use separate `--scratch` and `--output` directories for repetitions.
5. For the larger live ASR pack use `.\.venv\Scripts\python.exe -B benchmarks/asr.py --output benchmarks/output/stress rust --cli target/release/copilot.exe --model models/ggml-small.en.bin`. Keep that output separate from the 25-clip process comparison.

Keep foreground activity low. Model load is process-cold and file-cache state is uncontrolled. Archive A and B should use the same Windows hardware as C. The Python profiler bypasses the venv launcher when collecting counters.

For the separate desktop probe, retain the B desktop executable and run `powershell -NoProfile -File benchmarks/desktop-startup.ps1 -Before <baseline-desktop-exe> -After target/release/personal-copilot-desktop.exe`. It launches and closes isolated app instances and writes per-run window and process-tree measurements.

Do not commit private audio. The checked-in data is generated from the repository's synthetic scripts and Windows voices. Tests and profile commands need no paid credentials.
