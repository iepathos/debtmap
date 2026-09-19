# Coverage used for repository analysis

`just coverage-lcov` discovers all ordinary library, binary and integration tests
through Cargo. New integration targets are included automatically. The same
collection policy drives HTML, the 80% line-coverage threshold and CI.

This replaces a hand-maintained allowlist that selected 24 of 186 integration
targets locally and only 17 in CI. For example, the omitted framework suite's
eight tests raised `FrameworkDetector::matches_pattern` from 19/62 covered lines
(30.65%) to 61/62 (98.39%) in an incremental coverage check. The tests took 0.06 s;
the entire command, including an incremental build and report export, took
22.35 s. That is evidence of an omission, not a full-suite runtime benchmark.

## Commands and artifacts

| Command | Scope | Report |
| --- | --- | --- |
| `just coverage-lcov` | All ordinary test targets | `target/coverage/lcov.info` |
| `just coverage` | Same | `target/coverage/html/index.html` |
| `just coverage-check` | Same, minimum 80% line coverage | `target/coverage/coverage-summary.json` |
| `just coverage-fast-lcov` | Library tests only | `target/coverage-fast/lcov.info` |
| `just coverage-fast` | Library tests only | `target/coverage-fast/html/index.html` |

The `coverage-full`, `coverage-full-lcov` and `coverage-full-check` names remain
aliases for the representative defaults. `just analyze-self` collects fresh
representative LCOV before analyzing the repository.

Collection uses cargo-llvm-cov's libtest harness, with one test process per
binary rather than one per individual test. It enables all available features
in one configuration; it does not test every feature combination. Ordinary
`#[ignore]` annotations remain effective. Benchmarks, ignored diagnostics and
nightly-only doctest coverage are outside this scope.

After successful collection, `just coverage-report-lcov`,
`just coverage-report-html` and `just coverage-report-check` export another
format without rerunning tests. CI uses these same recipes and cargo-llvm-cov's
native threshold check, with no separate test list or threshold script.

Collection clears the previous completion marker, LCOV, JSON summary and HTML before
running tests. A failure prevents report export; report-only commands require a
successful collection marker. The marker records successful collection, not
freshness after subsequent source edits. Regenerate coverage after changing
code or tests.

Fast coverage uses `target/llvm-cov-fast-target` for instrumented artifacts and
profiles, separate from representative coverage. Running it cannot replace the
representative LCOV or mix partial profiles into a later representative report.
Use the representative report for debt rankings; a partial report only describes
its selected tests.

## Timing checks and prerequisites

Wall-clock thresholds are inappropriate under instrumentation. The coverage
lookup and boilerplate timing tests remain explicit ignored stress checks;
ordinary tests retain correctness checks for their underlying behavior.
`just test-stress` includes these timing checks in uninstrumented debug builds.
To run just those checks:

```sh
cargo test --test coverage_performance_regression_test \
  --test boilerplate_performance_test -- --ignored
```

Coverage requires `just`, `cargo-llvm-cov` and the toolchain's
`llvm-tools-preview` component. The Unix recipe integration tests also require
`just`; they use a temporary fake Cargo executable to verify discovery,
failure propagation and profile/report isolation without recursively building
the project. The coverage CI workflow installs the required tools.

Framework integration tests require the tracked `framework_patterns.toml`
fixture and now fail if it is missing instead of silently reporting success.

## Verification (2026-09-19)

The first expanded debug/offline collection passed 7,527 tests across 189 test
binaries, with 44 explicitly ignored tests. No integration targets were excluded
from discovery. LCOV generation took 412.24 seconds wall time, including
153 seconds of compilation. This is one observed run, not a warmed comparison
against the old selection.

The report contains 176,805 covered lines out of 206,922 executable lines
(85.4452%). `FrameworkDetector::matches_pattern` is 61/62 (98.3871%). These figures
describe the selected ordinary tests on this platform, not branch coverage or
every possible feature/platform combination.

The final targeted run passed all 19 ordinary coverage-policy, lookup,
boilerplate and framework tests; all seven retained timing checks passed
separately in the uninstrumented debug stress profile. The recipe contracts were
rerun after adding stale HTML cleanup. `just fmt`, offline `just test` (6,184
passed, ten existing skips), strict offline all-target/all-feature Clippy and
`git diff --check` also passed. Logs are under `/tmp/debtmap-coverage-scope-audit`.

The native 80% threshold check and HTML export completed successfully. A real
report-only isolation probe against the empty fast profile directory failed as
expected and left the representative LCOV unchanged.

With cargo-llvm-cov 0.6.21 and Rust 1.89, HTML export emitted 2,464 mismatched-data
warnings. Native LLVM diagnostics accounted for all of them: 16 distinct
zero-hash dependency functions repeated across binaries, with no Debtmap
symbols. The affected dependencies are foldhash, serde_json, itoa,
regex_automata, parking_lot and tracing; their sources are excluded from the
project report. This diagnostic remains visible rather than being suppressed.
The representative LCOV SHA-256 is
`b713614107b59acab4f00ddf1e00bea4d508b0af78f0e0ade6e6d9eb83b80c82`.
