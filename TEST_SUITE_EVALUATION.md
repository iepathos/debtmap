# Debtmap Test Suite Evaluation

Date: 2026-05-23

## Summary

Debtmap's test suite is broad and mostly healthy: the majority of tests are cheap, deterministic unit tests around pure parsing, scoring, classification, formatting, and data transformation logic. That aligns well with Stillwater's "pure core, imperative shell" philosophy.

The main test-cycle cost is not the count of tests by itself. A warm debug run showed:

- `cargo nextest run --lib`: 5,726 tests, 7 skipped, 24.9s nextest time.
- `cargo nextest run --test '*'`: 1,095 tests, 35 skipped, 8.7s nextest time.
- Old `just test`: 44.8s wall time.
- Combined equivalent `just test` selection: 5,771 tests, 7 skipped, 22.5s nextest time.

The old `just test` recipe paid avoidable overhead by running the lib suite and then a separate integration selection. The updated recipe keeps the same selected test scope in one nextest invocation and suppresses passing-test noise.

## Good Tests

These tests are worth keeping as normal fast-cycle tests:

- Pure classification and scoring matrices in modules such as `src/risk/priority/scoring.rs`, `src/risk/priority/recommendations.rs`, `src/priority/scoring/*`, `src/complexity/*`, and `src/analysis/*`.
- Parser and detector tests that feed inline source strings through a narrow function boundary, such as `tests/nesting_depth_comprehensive_test.rs`.
- Property tests around TUI/query/navigation state in `src/tui/results/*`; they exercise pure state transitions and are cheap.
- Boundary serialization/formatting tests that assert stable output contracts without invoking the CLI or filesystem.

These match Stillwater well: pure data in, pure data out, deterministic assertions, no external process setup.

## Redundant Or Low-Value Patterns

The suite has several places where coverage is duplicated across layers:

- `tests/context_aware_test.rs` and `tests/context_aware_integration_test.rs` cover closely named behavior at different layers. Keep the pure unit coverage broad and trim the integration file to a few pipeline-contract cases.
- `tests/method_function_disambiguation_test.rs` and `tests/method_function_disambiguation_integration_test.rs` overlap by intent. The integration file should only prove the parser/analyzer wiring, while name-resolution edge cases belong in focused unit tests.
- `tests/test_type_tracking.rs` and `tests/type_tracking_test.rs` should be consolidated or renamed by level, because their current names hide ownership and make targeted runs harder.
- Large table-style edge-case files such as `tests/core_untested_functions_tests.rs`, `tests/nesting_depth_comprehensive_test.rs`, `tests/cyclomatic_complexity_tests.rs`, and `tests/cognitive_complexity_tests.rs` are valuable, but many individual tests can become parameterized case tables to reduce boilerplate and compile surface.

## Slow Or Poorly Shaped Tests

These tests are the clearest candidates for moving out of the default cycle or simplifying:

- `src/risk/context/git_history/tests.rs`
  - `test_git_history_on_real_repo`: 15.8s.
  - `test_git_history_via_context_aggregator`: 2.6s.
  - `test_git_history_with_analysis_style_paths`: 2.2s.
  - These inspect the actual working repo and invoke real git history. They are high-value regression tests, but they are integration/stress tests and should be `#[ignore]` or moved behind an explicit recipe.

- `src/risk/context/mod.rs`
  - `test_context_aggregator_large_codebase`: 15.8s.
  - `test_large_call_graph_no_stack_overflow`: 3.6s.
  - These are stress/regression tests. Keep one small fast regression in the normal suite, and move the large versions to ignored/perf coverage.

- `tests/validate_parallel_test.rs`
  - Previously used `cargo run --bin debtmap` in six tests.
  - Now uses `env!("CARGO_BIN_EXE_debtmap")` to invoke the already-built binary.
  - Further improvement would be to test command handlers directly and keep one CLI smoke test.

- `tests/progress_display_integration_test.rs`
  - Several tests take 2.8-4.1s.
  - These should exercise progress rendering through pure state snapshots where possible; keep one subprocess or terminal-bound smoke test if needed.

- `tests/file_analysis_progress_test.rs`
  - Previously contained real sleeps and wall-clock assertions.
  - Now uses a deterministic pure throttling helper with simulated elapsed time.

## Stillwater-Aligned Recommendations

1. Keep the fast default cycle focused on pure core tests plus a small number of shell smoke tests.
2. Move remaining full CLI subprocess tests to named recipes such as `just test-cli` if they become a default-cycle bottleneck.
3. Prefer `env!("CARGO_BIN_EXE_debtmap")` over `cargo run` when a true CLI test is needed.
4. Refactor sleeps and wall-clock assertions into pure clock-input tests as they are encountered.
5. Consolidate similarly named integration/unit pairs so each layer has a clear reason to exist.
6. Use the existing `src/testkit` in-memory helpers more aggressively for filesystem-heavy tests.

## Implemented First Pass

Implemented:

- Marked the large git/context regressions as ignored slow tests.
- Added `just test-slow` for explicit git/context slow regression runs.
- Converted `tests/validate_parallel_test.rs` away from nested `cargo run`.
- Replaced progress sleep tests with a deterministic throttling helper.

Observed warm default cycle after this pass: `just test` completed in 8.3s with 5,766 passing tests and 12 skipped slow tests.
