# Coverage and scoring report corrections

The `Body::infer_at` report exposed four independent measurement problems:
closure-truncated coverage, recursive self-coupling, overconfident history risk,
and explanations that reconstructed a different scoring formula.

## Coverage

LCOV parsing now retains merged executable-line observations. AST-bound queries
count observed executable lines inside the inclusive source range, including
nested closures and excluding neighboring functions. Covered lines from repeated
records are unioned; function, file, and overall percentages use those merged
observations. Summary-only and function-only records retain their fallback.

The shared sequential/parallel scoring kernel binds all supplied metric bounds
once. Callee propagation and uncovered-line lookup use those same immutable
bounds. The registry uses normalized AST file/name/start identity; conflicting
columns or ends suppress attribution instead of falling back to fuzzy matching.
Queries without a matching AST identity retain the documented LCOV fallback.

Source lengths must share the recorded location's origin. Rust's direct analyzer
now counts from the identifier to the closing brace, matching cached extraction,
so preceding attributes/doc comments cannot push the end into a neighbor.
Expression closures use their original body span instead of a synthetic block.
JS/TS
callables use the full callable node rather than only its body, preserving the
end of functions with multiline signatures. Analyzer-level regressions cover
these prerequisites as well as coverage lookup itself.

The fallback used when AST bounds are unavailable skips nested callable symbols
when finding the next function boundary. It is still an inferred range: LCOV
start-only records cannot establish the exact closing line. Line data cannot
separate two definitions occupying the same physical line. A line covered by
any monomorphization is counted as covered; this is line coverage, not proof
that every branch or instantiation executed.

The HTML and LCOV recipes now include the six maintained resolution matrix/oracle
targets and the coupling regressions. Previously those tests ran in integration
and CI but were absent from the selected coverage suite.

The original artifact (`target/coverage/lcov.info`, SHA-256
`422c6aa7053179bc97d68ba22d583961dc91699ff300f4d1ac6f8e68fe62b629`)
contains 12 covered executable lines out of 33 in `infer_at`, lines 12–48.
Before the fix, its closure at line 32 truncated the parent to 5/19 (26.32%).
The repaired parser and bounded query both report 12/33 (36.36%) on that unchanged
artifact. This does not estimate coverage from the expanded suite.

## Dependencies and history

The graph keeps recursive edges. External coupling and scoring exclude self;
immediate-neighbor counts union definition IDs rather than summing incoming and
outgoing degrees. Colliding display names receive source-location details.
Historical records and aggregate records that lack original definition IDs can
only use their available labels. Degree alone no longer establishes
`critical_path`; human reports describe immediate neighbors, not transitive
change impact.

History retains the raw message-labelled fix ratio. Its risk contribution is
multiplied by `n / (n + 5)`, where `n` is the number of observed modifications
(excluding introduction for function history). Five prior-equivalent observations
are an explicit ranking policy, not an empirically calibrated probability. One
fix-labelled change now receives one sixth of the former history contribution.
Human output identifies the message-label heuristic and does not infer a ratio
denominator when history provenance is unavailable.

## Score explanations and compatibility

Live scoring records its numeric operations and actual operands. LLM, TUI,
Markdown, and verbose explanations consume that trace, including coverage,
configured weights, context, floors, exponents, and severity/risk multipliers.
The existing JSON `base_score` now means the actual pre-scaling score, rather
than the sum of display indicators. Records without a trace state that the
available factors cannot reconstruct the final arithmetic.

Neither legacy JSON v3 nor the existing receipt-bearing v4 gains fields. The arithmetic trace and exact immediate-neighbor
count are internal and excluded from serialization. No older unversioned binary
migration is claimed.

## Focused performance check

A small temporary Rust program called `parse_lcov_file` and queried `infer_at`
against the unchanged full repository LCOV artifact. The pre-fix parser binary
was retained before implementation. Each binary ran one excluded warmup and five
measured runs using `/usr/bin/time -l`, serially with no concurrent project builds
or tests. Raw receipts and the temporary probe are under
`/tmp/debtmap-report-verification`; no benchmark runner was added to the project.

| Parser | Five wall times (seconds) | Median | Median peak RSS (bytes) |
| --- | --- | --- | --- |
| Before | 1.44, 1.46, 1.45, 1.46, 1.45 | 1.45 | 70,696,960 |
| After | 0.86, 0.85, 0.85, 0.86, 0.85 | 0.85 | 76,365,824 |

Retaining executable-line data adds about 8.02% peak RSS in this focused check.
Coverage is computed once after all records are merged, avoiding repeated
per-record work and tiny per-function parallel jobs. These measurements cover
parsing and lookup only; they do not establish end-to-end scoring performance.
The existing Criterion coverage benchmark now also exercises exact AST-bound
lookups through absolute-to-relative path matching.

## Final verification (2026-09-19)

All checks used debug builds and offline dependencies:

- `just fmt` and `just test`: 6,184 tests passed; 10 existing skips.
- `just test-integration`: 541 passed; one existing skip, including all maintained
  resolution matrices and the new recursive dependency regressions.
- Seventeen additional coverage, scoring, output and parallel-analysis contract
  targets: 86 passed; five existing skips.
- `cargo clippy --offline --all-targets --all-features -- -D warnings`: passed.
- `just coverage-lcov`: passed with the expanded integration selection.
- The debug Criterion `indexed_lookup_with_ast_bounds` case measured
  8.7294–9.3187 ms for 2,000 queries (central estimate 8.9527 ms).

The final CLI verification used:

```sh
cargo build --offline --bin debtmap
/usr/bin/time -l target/debug/debtmap analyze . --no-tui --format json --min-score 0 \
  --profile --profile-output /tmp/debtmap-report-verification/report.profile.json \
  --context --lcov target/coverage/lcov.info \
  > /tmp/debtmap-report-verification/report.json \
  2> /tmp/debtmap-report-verification/report.stderr
```

The receipt records 911 analyzed files, zero failures and complete scope. The
existing CLI format version remains 4.0; the legacy v3 output contracts also pass.
The minimum score is explicitly zero so the repaired finding remains inspectable.

| `Body::infer_at` | Original supplied report | Final verification |
| --- | --- | --- |
| Direct / transitive coverage | 26% / 26% | 81.82% / 81.82% |
| External callers / callees | 5 / 11 (included self) | 4 / 10 |
| Production neighborhood | 16 (summed degrees) | 11 distinct definitions |
| Coupling classification | Hub | Connector |
| Critical path | Yes (degree heuristic) | No claim |
| Contextual risk multiplier | 2.71 | 1.2857 |
| Final score | 44.65 | 3.34 |

The coverage difference combines a measurement fix (26.32% → 36.36% on unchanged
data) and expanded test execution (81.82%). It does not mean the resolver became
more accurate in this change, or that score reduction alone demonstrates quality.
The final LCOV SHA-256 is
`c7b8502fd8d3ddea5519803dc05da421091d0ae6c73ac193fcd7f7fd86c8858b`.

The single CLI run took 106.15 s wall time and reported peak RSS of 2,301,067,264
bytes. Its profile recorded 27.846 s graph construction, 51.422 s context loading
and 15.985 s debt scoring. This is an execution check, not a matched before/after
benchmark; it supports no repository-scale performance improvement claim.

The implementation was separated into source-range, coverage, dependency,
score-explanation and history commits. Each intermediate revision passed
`just fmt`, offline `just test` and commit hooks in an isolated worktree. Fast
suite counts were respectively 6,160, 6,175, 6,175, 6,180 and 6,184, each with
10 existing skips. The coverage revision also passed 26 focused tests; the
dependency revision passed 17 recursion/output tests; and the explanation
revision passed 23 output/formatter tests. The reconstructed source, tests and
configuration match the combined verified implementation exactly.
