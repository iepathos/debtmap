#!/usr/bin/env python3
"""Measure five warmed debug CLI runs on staged, verified source fixtures.

Run from the repository root on macOS. Sources are copied outside repository
exclusions, and the warm-up receipt must confirm all three files were analyzed.
No builds, downloads, or source-fixture compilation are performed.
"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import statistics
import subprocess
import tempfile
import time


def measure(command, corpus):
    start = time.perf_counter()
    result = subprocess.run(
        ["/usr/bin/time", "-l", *command], cwd=corpus,
        stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, check=True,
    )
    elapsed = time.perf_counter() - start
    peak = next(
        int(line.split()[0]) for line in result.stderr.splitlines()
        if "maximum resident set size" in line
    )
    return {"seconds": round(elapsed, 6), "peak_rss_bytes": peak}


def benchmark(binary, corpus, sources):
    binary = Path(binary).resolve()
    command = [str(binary), "analyze", ".", "--no-parallel", "--no-tui", "--quiet", "--format", "json"]
    warmup = subprocess.run(command, cwd=corpus, capture_output=True, text=True, check=True)
    scope = json.loads(warmup.stdout)["receipt"]["scope"]
    if (scope["analyzed_files"] != len(sources) or scope["failed_files"] != 0
            or scope["total_loc"] != 89
            or scope["code_breakdown"]["production_functions"] != 64):
        raise RuntimeError(f"Benchmark inputs were not analyzed: {scope}")
    runs = [measure(command, corpus) for _ in range(5)]
    return {
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "command": command, "working_directory": str(corpus), "verified_scope": scope,
        "runs": runs,
        "median_seconds": statistics.median(run["seconds"] for run in runs),
        "maximum_peak_rss_bytes": max(run["peak_rss_bytes"] for run in runs),
        "fixture_sha256": {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in sources},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/debug/debtmap")
    parser.add_argument("--output", required=True)
    parser.add_argument("--baseline-binary")
    parser.add_argument("--baseline-output")
    args = parser.parse_args()
    if bool(args.baseline_binary) != bool(args.baseline_output):
        parser.error("--baseline-binary and --baseline-output must be supplied together")
    sources = sorted(Path("tests/data/rust_method_resolution").glob("*.rs"))
    with tempfile.TemporaryDirectory(prefix="debtmap-method-corpus-") as directory:
        corpus = Path(directory)
        for path in sources:
            shutil.copyfile(path, corpus / path.name)
        if args.baseline_binary:
            baseline = benchmark(args.baseline_binary, corpus, sources)
            Path(args.baseline_output).write_text(json.dumps(baseline, indent=2) + "\n")
        report = benchmark(args.binary, corpus, sources)
        Path(args.output).write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
