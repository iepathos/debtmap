# Nine Rust resolution repairs: validation and performance

Measured on 2026-09-12 using debug builds on an Apple M2 Pro, 16 GiB RAM,
macOS 15.5 (24F74), rustc 1.89.0.

The final repository median is 100.120 seconds versus 94.514 seconds for the
scoring-fixed baseline (+5.93%). The synthetic median falls from 33.354 to
26.774 seconds (−19.73%). Median peak RSS decreases 9.46% and 9.42% respectively.
The highest synthetic peak RSS increases 17.38%; the highest repository peak
decreases 9.31%. Both final total-runtime and memory comparisons stay within the
requested investigation thresholds.

## Inputs and method

- Baseline: `095585fe`, after the Git-history negative cache/source-name and live
  scoring-progress fixes.
- Initial complete repair: `8af53cc7`.
- Final resolver: `922fa88f`, including the lookup and graph-merge optimizations.
- Repository input: frozen worktree at `095585fe`, 883 analyzed files,
  332,202 LOC, 9,408 production functions and 6,315 test functions. The same existing
  LCOV file was copied into its `target/coverage/lcov.info`.
- Synthetic input: 401 parse-only Rust files, 10,001 LOC, 9,201 functions. Four
  hundred modules each declare an owner method, helper, start function and 20
  cross-module step functions. The root declares all modules and calls the last.
  No Cargo project build is involved.
- Each series uses one excluded warmup and five measured runs, executed serially
  without concurrent builds. Source inputs and binaries remain frozen. Every
  receipt reports the same scope and zero failed files.
- Repository receipts confirm requested context and successfully loaded LCOV.
  Synthetic runs disable context and omit coverage.
- Wall time surrounds execution of the CLI; output parsing and input setup are
  outside it. Peak RSS comes from macOS `/usr/bin/time -l`, in bytes. The table
  converts bytes to MiB. Phase medians are independently calculated and are not
  additive because some phases contain others.

[Machine-readable results](rust-resolution-nine-gaps.json) preserve source and
binary SHA-256 fingerprints, exact per-series commands, scope, every measured
run's timings and memory, and phase medians. The JSON key `final` identifies the
initial complete repair; `optimized-final` identifies the delivered resolver.

## Measurements

| Series | Total s | Graph s | History preload s | Function scoring s | Median peak MiB | Highest peak MiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Repository baseline | 94.514 | 22.563 | 44.905 | 6.167 | 1988.5 | 2079.4 |
| Repository initial repair | 105.152 | 33.424 | 44.697 | 6.275 | 1875.5 | 2061.7 |
| Repository final | 100.120 | 27.372 | 44.559 | 6.370 | 1800.3 | 1885.7 |
| Synthetic baseline | 33.354 | 11.311 | — | 0.084 | 322.7 | 325.9 |
| Synthetic initial repair | 36.142 | 12.198 | — | 0.093 | 393.1 | 432.2 |
| Synthetic final | 26.774 | 2.546 | — | 0.100 | 292.3 | 382.6 |

| Series | Run 1 s | Run 2 s | Run 3 s | Run 4 s | Run 5 s |
| --- | ---: | ---: | ---: | ---: | ---: |
| Repository baseline | 94.214 | 95.950 | 94.043 | 94.514 | 95.871 |
| Repository initial repair | 105.691 | 105.152 | 104.211 | 103.864 | 106.244 |
| Repository final | 100.964 | 98.872 | 98.832 | 101.019 | 100.120 |
| Synthetic baseline | 32.986 | 33.222 | 33.354 | 33.369 | 33.389 |
| Synthetic initial repair | 36.185 | 36.238 | 36.079 | 36.067 | 36.142 |
| Synthetic final | 26.437 | 26.553 | 26.774 | 26.910 | 26.869 |

The repaired repository scoring remains near baseline (6.370 versus 6.167 seconds,
+3.28%), and history preload remains near 45 seconds. Final repository graph
construction is still 21.32% slower than baseline (27.372 versus 22.563 seconds);
the complete workspace and additional identity/constraint work have a measurable
cost even though total runtime is within the requested 10% threshold. Synthetic
function scoring increases from 0.084 to 0.100 seconds; its absolute cost remains
small, while graph construction falls from 11.311 to 2.546 seconds.

## Investigation and resulting changes

The initial repair exceeded the repository total-runtime threshold (+11.25%) and
the synthetic median-memory threshold (+21.85%; highest peak +32.59%). These
triggered profiling and implementation changes before acceptance.

A separate 30-second sampling profile of the repository run identified repeated
namespace resolution, callable lookup and workspace-membership checks during body
analysis. Parsing was a smaller contributor; attributing all overhead to the
second parse would have been incorrect. Synthetic diagnostic runs had the same
9,201 graph definitions and 8,801 resolved call sites, with no ambiguous or
unresolved sites in either version, so additional graph outcomes did not explain
the memory increase.

The final index caches exact callable paths, raw trait declaration candidates, and
unambiguous workspace membership. Canonical cached-source graph merges preserve
metric precedence and role evidence without rebuilding two complete graph maps.
Legacy and ambiguous identity handling retain their original fallback. See
[lookup performance changes](../rust-resolution-lookup-performance.md).

Raw stdout, stderr, profiles and the separate sampling trace were retained locally
under `/tmp/debtmap-nine-perf`; they are not repository fixtures.

## Reproducing the measurements

The actual repository command, run from `/tmp/debtmap-nine-benchmark`, was:

```sh
/usr/bin/time -l /tmp/debtmap-nine-optimized analyze . --no-tui --format json \
  --profile --profile-output /tmp/debtmap-nine-perf/optimized-final/run-1.profile.json \
  --context --lcov target/coverage/lcov.info \
  > /tmp/debtmap-nine-perf/optimized-final/run-1.json \
  2> /tmp/debtmap-nine-perf/optimized-final/run-1.stderr
```

For the baseline, substitute `/tmp/debtmap-scoring-baseline` and the baseline output
directory. For the synthetic series, run in `/tmp/debtmap-nine-synthetic` and
replace the context/LCOV arguments with `--no-context-aware`. Execute an excluded
warmup, then runs 1–5 with distinct output filenames. On Linux, `/usr/bin/time -v`
reports maximum RSS in KiB rather than bytes.

The measurements above were collected before the temporary measurement scripts
were removed. There is no maintained Python or custom Rust benchmark runner.
Repository-scale CLI timing uses the command above. Resolver benchmarks use the
existing Criterion dependency and `call_graph_bench` target:

```sh
cargo bench --offline --profile dev --bench call_graph_bench -- rust_workspace_resolution
```

This explicitly selects debug builds. Criterion measures sequential and parallel
workspace construction at 199, 200, 201 and 401 files; source generation happens
outside the timed closure, and a preliminary graph check verifies definition and
resolved-site counts with no uncertainty. This smaller boundary workload differs
from the 9,201-function synthetic CLI workload above. Criterion's `--test` mode
checks that each benchmark executes without claiming a statistical measurement.
All eight cases also completed a Criterion measurement run with
`--sample-size 10 --warm-up-time 1 --measurement-time 1 --noplot`. Criterion
extended collection where ten samples required more than one second. These
measurements validate the standard benchmark workflow; they are not a second
baseline/final comparison.

## Accuracy and compatibility validation

The permanent tests cover direct AST, cached-source, sequential and parallel
extraction, enhancement and repeated merges. Location-aware assertions distinguish
same-line definitions and reject forbidden resolved and possible targets.
Sequential/parallel comparisons include nodes, metrics, role evidence, edge
evidence and uncertainty, with original, reversed and deterministic shuffled file
orders at the four batch boundaries.

Validation passed in debug builds:

- `just fmt` and `CARGO_NET_OFFLINE=true just test`: 6,140 passed, 10 existing skips.
- Strict offline all-target/all-feature Clippy with warnings denied.
- 62 focused resolver/identity/batching/consumer integration tests.
- 97 output-contract, extraction, language and integration tests across 15 targets.
- All eight Criterion workspace cases pass their execution checks with
  `cargo bench --offline --profile dev --bench call_graph_bench -- rust_workspace_resolution --test`.

Legacy JSON defaults missing columns to absent. Current FunctionId and extraction
Postcard records round-trip; migration of old unversioned binary layouts is not
claimed. FunctionMetrics legacy JSON is covered; graph and FunctionMetrics
Postcard serialization are not newly promised. Public JSON v3 gains no fields.
Possible edges remain excluded from resolved counts and coverage; impossible
targets receive no dead-code protection in the covered regressions.

These measurements describe one machine and these exact debug inputs. Fixture
precision and boundary benchmarks do not establish general Rust precision or
repository-scale performance. Default trait-body instantiation, custom Deref,
general trait solving, arbitrary argument inference, block-local declaration
indexing and fixed-point loop analysis remain deferred. The existing basename
restriction for renamed free-function imports is also documented.
