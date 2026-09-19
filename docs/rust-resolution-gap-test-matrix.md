# Rust resolution gap test matrix

This is a tests-only baseline against `105ec3dc`, recorded on 2026-09-13. It
specifies the behavior required before designing another repair. Production code
is unchanged. The new correctness failures remain active and unignored; a green
existing suite does not mean these gaps are closed. The user explicitly requested
committing this failing specification baseline before planning production repairs.

## Contracts and dimensions

The matrices test three shared contracts instead of only the previously reported
expression spellings:

1. **Identity migration preserves definitions.** A legacy ID can upgrade only
   when its file, qualified source name and line identify a unique definition.
   Exact columns distinguish same-line definitions. A mismatched write must not
   replace another definition's sentinel facts, including either endpoint of a
   transformation.
2. **Expression changes preserve receiver constraints.** Parentheses, references,
   dereferences and propagation must not admit unrelated owners. Qualification
   provides positive controls. Unsupported block-local declarations remain
   constrained uncertainty, rather than falling back to a global namesake.
3. **Value lookup accounts for every admissible binding.** Type-only namesakes
   must not compete with constructor values. Local and explicit bindings take
   precedence over globs. Ambiguous values must not become a resolved function
   merely because another binding has no callable body node.

| Matrix | Dimensions |
| --- | --- |
| Identity | Exact, unique legacy, foreign file, different line/name/owner/module-qualified name/column, and same-line tie; purity, dependencies, I/O, mutation, CFG and CFG context; current and legacy JSON; append operations; independent transformation endpoint products |
| Receiver expressions | Six forms (`x`, `(x)`, `*x`, `&*x`, `*(&x)`, `**(&x)`) crossed with shadowed/qualified dynamic and nominal types, and primitive/alias receivers |
| Generic syntax | Plain, tuple and reference type arguments; local shadow versus explicit `crate::` name; `W::<T>::hit()` versus `<W<T>>::hit()`; qualified/unqualified trait paths and arguments |
| Propagation | Returned dynamic bounds and aliases in a different lexical module; copy, tuple, block tail and branch join; constructor fields and returned fields; dereferenced variants |
| Namespace lookup | Unit/tuple constructors crossed with aliases, enums, braced structs, unions and traits; functions/constants; local/explicit/glob precedence; renamed imports, reexports, duplicate imports and conflicting globs |

The dimensions are selected products, not an exhaustive Cartesian product of
Rust syntax. In particular, tests for unsupported block-local aliases require
uncertainty with no unrelated targets; they do not require full local indexing.
Generic trait cases likewise preserve admissible candidates without requiring
expanded trait solving.

## Independent expectations and extraction paths

Fixtures mark expected callable definitions and call sites in source text. The
oracle computes exact file, line and column locations from those markers, without
using resolver output or matching display names. Every row specifies exact
resolved or possible target sets, or absence of a callable outcome (for example,
a constructor without a callable body). Checks also reject ghost nodes, extra
sites and duplicate site outcomes. Callers and targets must also be exact
members of the graph, not merely have a matching location. Some rows require a specific uncertainty
reason; full uncertainty metadata is always compared for builder parity.

Every source row runs through:

- Direct multi-file AST extraction.
- Direct extraction followed by trait/framework enhancement and repeated finalization.
- Cached extraction adapter.
- Cached graph builder.
- Sequential source builder seeded with extracted metrics.
- Parallel source builder seeded with the same metrics and reversed file order.

Each graph is checked again after two repeated merges, producing twelve graph
states per row. Sequential/parallel parity and merge idempotence compare nodes,
function metadata, roles, role evidence, calls, edge evidence and uncertainty.
The fixtures use identical source contents across these paths.

The compiler oracle independently checks the exact 113 generated sources using
standalone `rustc --edition=2024 --crate-type=lib --emit=metadata`. All 106 valid
sources must compile. The seven deliberately ambiguous inputs must fail with
E0659. This checks source validity and ambiguity classification; it does not
derive expected call targets from the compiler. No fixture Cargo project is
built, no fixture is executed, and no dependencies are downloaded.

Five harness rejection tests deliberately inject incorrect graphs. Layout
and ordering checks cover five source transformations (including same-line
definitions, an inline module and an unrelated owner) and three file orders.
The existing workspace regressions retain 199/200/201/401-file boundary and
captured-source checks.

## Baseline census

Counts below are matrix cells, not separately registered Rust test functions.
Each group evaluates every cell before asserting its aggregate result.

| Group | Cells | Passing | Failing |
| --- | ---: | ---: | ---: |
| Fact writes and current JSON roundtrips | 90 | 70 | 20 |
| Legacy fact JSON | 28 | 28 | 0 |
| Transformation writes and current JSON roundtrips | 162 | 138 | 24 |
| Legacy transformation JSON | 49 | 49 | 0 |
| Raw current JSON with mixed-column endpoints | 9 | 9 | 0 |
| Receiver forms and qualification | 36 | 30 | 6 |
| Generic owner and trait arguments | 16 | 11 | 5 |
| Return, field and binding propagation | 22 | 19 | 3 |
| Valid namespace lookup | 32 | 32 | 0 |
| Deliberately ambiguous namespace lookup | 7 | 2 | 5 |
| **Total** | **451** | **388** | **63** |

The failing regions are:

- Foreign-file and different-line legacy writes contaminate canonical facts and
  transformations. There are 44 failing identity cells, with 180 failed checks
  because each cell checks several reads and sentinels.
- Shadowed dynamic/nominal receiver constraints are lost through direct
  dereference, reborrow and double dereference: six cells.
- Shadowed generic arguments bypass constraints in `W::<T>` syntax (three
  argument shapes) and in two trait-path forms: five cells.
- Tuple dereference, branch-join dereference and constructor-field dereference
  lose local-shadow constraints: three cells.
- Competing unit constructors, competing tuple constructors, function/tuple,
  function/unit and constant/unit globs mishandle ambiguity or admissible
  owners: five cells.

These are related failures across combinations, not 63 independent root causes.
The passing compiler classifications, layout/order checks and harness rejection
tests are additional validation and are not included in the 451-cell census.

## Reproduction

Use the normal Cargo integration test runner. `--no-fail-fast` is necessary to
continue to the other binaries after an expected failing matrix:

```sh
cargo test --offline --no-fail-fast \
  --test data_flow_identity_matrix \
  --test rust_resolution_constraint_matrix \
  --test rust_resolution_namespace_matrix \
  --test rust_resolution_matrix_oracles \
  --test rust_resolution_matrix_invariants \
  --test rust_resolution_matrix_assertions -- --nocapture
```

Expected baseline: exit 101, with failures in the identity, constraint and
namespace matrix targets. Case labels and source text accompany failures.

```sh
just fmt
CARGO_NET_OFFLINE=true just test
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo test --offline \
  --test rust_enhancement_lifecycle \
  --test data_flow_legacy_identity \
  --test rust_method_resolution_review_followup \
  --test rust_workspace_batches
```

`just test` uses the repository's selected fast suite and does not include the
new matrices. Its passing result must be reported separately from the red
correctness baseline. All commands use debug builds. There are no Python scripts,
custom runners or new dependencies.

Recorded validation: formatting and strict all-target/all-feature Clippy pass;
`just test` passes 6,140 tests with 10 existing skips; the four existing targeted
regression binaries pass 17 tests. The combined new suite evaluates 19 aggregate
tests: 13 pass and six fail across the three expected failing targets. The two
compiler tests cover all 113 classifications, the two layout/order tests cover
eight variants, and all five harness rejection tests pass.

## Limits and the next repair's acceptance rule

This matrix targets the gaps found after the previous repairs. It supplements
the existing nine-gap, JSON v3, Postcard, consumer, enhancement and workspace
regressions; it does not replace them. New identity serialization products cover
JSON and omit CFG fields that the format intentionally skips. Existing tests
remain responsible for current Postcard compatibility; old unversioned binary
migration is not promised.

Readability of out-of-graph legacy records and policy for standalone
`module_path` metadata remain unspecified. Qualified source names are tested.
Full block-local indexing, arbitrary generic inference, default trait-body
instantiation, custom `Deref`, general macros and fixed-point loop analysis
remain outside this repair. The compiler classifications cover the generated
single-file constraint/namespace fixtures, not every layout or multi-file
variant. No repository performance or general Rust precision claim follows
from this small corpus.

A subsequent repair must make the active desired-behavior matrices green while
preserving positive controls and the existing regressions. Any necessary change
to an expectation must explain the semantic reason independently of current
implementation behavior. Solution design and production changes follow this
baseline as a separate step.

## Repair validation, 2026-09-19

The original **451 cells now pass**; all 63 baseline failures are closed without
ignores or weakened expectations. The baseline census above is retained as the
historical reproduction. Twenty additional tests cover the new helper contracts,
legacy module metadata, value-result propagation, constructor argument visits,
and downstream liveness exclusions.

The repairs share exact legacy identity matching, structural body-scoped type
lowering, fact-based associated lookup and complete value-binding collection.
Unsupported type syntax and dynamic-bound arguments retain their previous
shadow exclusions. Alias, field and return expansion keeps declaration scope.
The previously unspecified module-metadata contract now has explicit tests:
missing metadata permits a unique exact source match, populated metadata must
agree, and unmatched records keep their original storage identity.

Final validation: `just fmt`; strict offline all-target/all-feature Clippy;
`CARGO_NET_OFFLINE=true just test` (6,154 pass, 10 existing skips); expanded
`just test-integration` (531 pass, one existing skip); and 169 targeted
compatibility/consumer regressions across 23 binaries. Integration validation
includes all 113 compiler classifications, five harness rejection tests and
eight layout/order variants. A duplicated internal test-module inclusion and
unused internal AST adapters were corrected before the final clean gates.

The six matrix/oracle targets are now included in the local integration recipe
and the CI integration job. The fast suite remains selected; use the integration
gate to exercise the complete matrix. The existing solver limits above remain
in effect. Performance results are recorded separately in the
[repair benchmark report](benchmarks/rust-resolution-gap-repairs.md).

## Constructor diagnostic regression specification, 2026-09-19

A subsequent review found an uncovered combination: a known tuple constructor
and an unresolved explicit import with the same name. Added an active regression
test requiring exactly one `AmbiguousDeclaration` call record with no known
targets. It also requires a nested argument call to resolve exactly once.
A companion control requires two known explicit constructor bindings to omit
constructor diagnostics while still visiting the argument once.

Both use the existing six-path harness, exact source identities, metadata parity,
and repeated merges. These deliberately conflicting/incomplete sources are
parse-only specifications, separate from the compiler-classified matrix rows.
No production behavior changed. The original 39 namespace rows still pass; the
namespace target now has five passing tests and one expected regression failure.
The failure is the missing call record in every path, not a wrong target or
duplicate argument visit. The failing test is neither ignored nor inverted and
is already included in the local and CI integration gates.

```sh
cargo test --offline --test rust_resolution_namespace_matrix
```
