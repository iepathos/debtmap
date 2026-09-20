# Rust effect analysis

Debtmap's Rust purity signal is derived from owned semantic evidence collected by the bounded workspace resolver. Observed effects, unresolved behavior, and provenance are retained separately through graph merging, propagation, data-flow preparation, scoring, and human reporting.

## Classification contract

| Evidence | Internal classification |
| --- | --- |
| Complete supported analysis, no effects | `StrictlyPure` |
| Complete analysis, local mutations only | `LocallyPure` |
| Complete analysis, external reads without writes or I/O | `ReadOnly` |
| Confirmed external write, I/O, or modeled nondeterminism | `Impure` |
| Incomplete analysis without a confirmed impure effect | `Unknown` |

Confirmed impure effects remain `Impure` even when additional behavior is unresolved. Every observed fact and uncertainty reason remains available for explanations. The model concerns modeled program effects; it does not prove termination, panic freedom, allocation success, or timing independence.

## Supported evidence

Project calls use exact declaration identities, lexical scopes, receiver facts, and source locations. Callable references remain reachability relationships and do not execute their targets. Closure bodies contribute effects only when a supported invocation relationship executes them. Directly bound callbacks and modeled `Option`/`Result` callback operations are supported; unsupported consumers, escapes, reassignment, and indirect invocation remain uncertain.

Transparent project wrappers are recognized from a single direct callback-parameter invocation, not from a function-name whitelist. Their callers retain confirmed callback effects alongside any unresolved generic-wrapper behavior. Immutable locally bound closures can execute against their captured lexical facts; changed captures and unsupported forwarding remain uncertain. Constructing supported async functions, methods, blocks, or closures does not execute their bodies.

The reviewed standard-library registry covers the primitive and string operations needed by the resolver corpus, selected collection operations, selected `Option`/`Result` operations, memory-backed cursor reads/writes, and explicit filesystem, console, environment, time, and socket operations. Collection mutation is local only when receiver ownership is established. Generic dispatch such as an unknown key's `Hash` implementation remains uncertain.

Project definitions take precedence over library models. Imports, aliases, lexical shadowing, receiver facts, and source identity determine model selection; variable spelling and method-name substrings do not. Unsupported third-party HTTP/database APIs remain `Unknown`, not inferred I/O.

## Compatibility and scoring

Public JSON v3/v4 fields and enum values are unchanged. Internal `Unknown` uses the existing optional-field omission behavior. Only `StrictlyPure` maps to the historical `is_pure: true`; internal consumers use the full assessment.

Unknown receives neutral complexity and data-flow factors of 1.0. Data-flow purity factors are 0.0 for `StrictlyPure`, 0.3 for `LocallyPure`, and 1.0 for `ReadOnly`, `Impure`, and `Unknown`. Fresh analysis does not infer the legacy `IOIsolated` or `IOMixed` discounts from operation counts.

Persisted purity records include an evidence/model version. Obsolete or unframed cache payloads are rebuilt, and absent evidence remains unknown rather than inheriting a legacy purity label.

Live scoring traces list observed effects separately from unresolved behavior. Public compatibility records without evidence do not synthesize side-effect explanations from a purity label; loading an older report cannot reconstruct missing provenance.

## Known limits

Debtmap does not perform Cargo builds, rust-analyzer integration, general macro expansion, arbitrary trait-dispatch execution, custom `Deref`/operator reasoning, third-party API modeling, or structural I/O-isolation solving. Possible call targets remain possibilities and do not become observed execution. These limits can withhold a purity discount without erasing confirmed effects.

Static access, unsupported dereference/index/iterator operations, implicit destruction of owned generic or project values, and unsupported callback consumers prevent a completeness claim. Memory-backed cursor operations are not external I/O, but unsupported generic dispatch can still make their assessment unknown. This intentionally favors withholding a discount over claiming unsupported purity.

Library models do not inherit completeness through arbitrary argument conversions such as user-defined `AsRef`. Console models retain formatting-dispatch uncertainty alongside their confirmed I/O. Uncertain receiver identities cannot select confirmed standard-library models.

## Accuracy validation

The LCOV-backed report for `CallResolver::resolve_call_outcome` reports unknown purity, neutral complexity/data-flow purity factors, retained local mutation evidence, and separate unresolved operations. It makes no external-read, external-write, or network/I/O claim for that resolver. Its final score is not an acceptance criterion.

Validation includes the classification join matrix; property tests for normalized joins and uncertainty; callable references, bound closures and wrapper variants; shadowed/aliased models; JSON and Postcard evidence round trips; public v3/v4 contracts; both propagation paths; and direct/cached, sequential/parallel workspace cases at 199, 200, 201, and 401 files.

The initial offline debug gates passed: 6,252 fast tests (ten existing skips), 541 bounded integration tests (one existing skip), 58 focused pipeline tests, formatting, and strict Clippy across all targets and features. These bounded suites did not cover every integration target run by `just coverage-lcov`.

The subsequent coverage run exposed lost source queries for uncertain shadowed calls and stale role/model expectations. The follow-up preserves the original callable query without resolving the call, makes absolute standard-library type aliases agree with directly qualified types, and checks missing evidence as `Unknown`. The three affected integration targets pass all 33 tests.

Follow-up validation on 2026-09-19 ran the actual `CARGO_NET_OFFLINE=true just coverage-lcov`: 7,602 tests passed across 192 binaries, zero failed, and 44 existing tests were ignored. LCOV export completed successfully at `target/coverage/lcov.info`; no tests were excluded and the recipe was unchanged. Formatting, all 6,252 fast tests (ten existing skips), and strict offline all-target/all-feature Clippy also passed. The originally reported library failure did not reproduce in ordinary libtest, the instrumented library binary, or the full coverage run; its cause remains unconfirmed.

### Debug microbenchmarks

Criterion was run with `cargo bench --offline --profile dev --bench call_graph_bench -- effect --sample-size 10 --warm-up-time 1 --measurement-time 2` on an arm64, 12-core, 16-GiB machine using Rust 1.89.0. Estimates are local measurements, not cross-machine performance guarantees.

| Benchmark | Estimate |
| --- | ---: |
| Construct/deduplicate 100 evidence records | 196.11 µs |
| Resolve and collect evidence for 100 functions | 4.9894 ms |
| Propagate an I/O effect through 100 functions | 2.2834 ms |
| Score one evidence-bearing function | 144.10 µs |

Propagation includes graph/adapter setup. The new effect benchmarks have no equivalent pre-evidence baseline; their earlier development snapshots are not before/after measurements of the old analyzer.

### Repository performance comparison

Both debug binaries analyzed the same clean, fixed worktree at `8b650ea915dda700969b09220e4156af4b51b16b`, with context, the existing LCOV snapshot, `--no-tui`, JSON output, and profiling. Five final paired runs followed warm-up analyses; each measurement started a fresh process. Development samples taken before the final corrections were excluded. No Debtmap builds or tests ran alongside the final measurements.

| Median measurement | Baseline | Evidence analysis | Change |
| --- | ---: | ---: | ---: |
| Total wall time | 94.54 s | 111.44 s | +17.9% |
| Graph building and effect collection | 27.419 s | 40.983 s | +49.5% |
| Purity propagation | 1.880 s | 2.263 s | +20.4% |
| Git-history preload | 46.926 s | 51.846 s | +10.5% |
| Debt scoring | 9.169 s | 10.347 s | +12.8% |
| Maximum resident set size | 1.497 GiB | 1.603 GiB | +7.0% |
| macOS reported peak memory footprint | 1.205 GiB | 2.537 GiB | +110.4% |

The five wall-time samples were baseline `90.51, 97.06, 101.69, 94.54, 94.29` seconds and candidate `109.75, 102.58, 119.93, 111.44, 118.22` seconds. Phase medians are independent and need not sum to total time. Both memory measures are reported because they differ substantially; RSS alone would conceal the footprint increase.

The runtime and footprint investigation thresholds were exceeded. Investigation removed redundant propagation, replaced copied transitive explanation chains with compact call-site/category summaries, and reused owned maps during collection/propagation joins. Remaining costs include the new semantic collection itself and duplicated assessments across propagation snapshots, graph copies, and per-function data-flow records. Record keys also duplicate owned source identities. These are code-review findings, not allocation-profile attribution; an allocator-level profile was not performed. Git-history variability also affects whole-run timings. The remaining runtime/footprint regressions are explicitly unresolved performance limitations, not claimed passes of the thresholds.

Reproduce each measurement from the fixed input worktree with the respective debug binary:

```sh
/usr/bin/time -l debtmap analyze . --context --coverage-file /path/to/lcov.info \
  --no-tui --min-score 0 --format json --output /path/to/report.json \
  --profile --profile-output /path/to/profile.json
```

The LCOV snapshot SHA-256 was `a8e4de9f57c611327d903bc37d6fce88fd43e6e29f4f18f95b492e5c38b4e0f4`. The final sorted `src` file SHA-256 manifest digest was `35480f131d8e5dc24372869a8ceee5c2a9eb92f46a2cd4f32234d664de29837a`. Local raw timing/profile receipts were retained under `/tmp/debtmap-effect-validation/paired-*`, final run identifiers 4–8.
