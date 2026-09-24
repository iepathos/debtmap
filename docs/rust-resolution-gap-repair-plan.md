# Repair plan for the systematic resolution matrix

The test baseline is committed as `36d6f670`: 451 cells, originally 388 passing
and 63 failing, with production behavior from `105ec3dc`. This document preserves
the repair design and records implementation status separately below.

Implementation update, 2026-09-19: stages 97–101 are complete and all original
451 matrix cells pass. Final formatting, Clippy, fast/integration and targeted
compatibility gates pass. Five-run debug CLI comparisons and all eight Criterion
boundary cases stay within the investigation thresholds. See the
[matrix validation appendix](rust-resolution-gap-test-matrix.md#repair-validation-2026-09-19)
and [benchmark report](benchmarks/rust-resolution-gap-repairs.md) for results and
the host-contention limitation on repository timings. The
design and expected stage counts below are preserved as planning history.

## Diagnosis and implementation order

Code review traces the failures to four shared defects. Repair them at the point
where information is first lost, and keep inference and graph recording on the
same resolution path.

| Stage | Shared defect | Failing cells addressed | Expected remaining failures |
| --- | --- | ---: | ---: |
| 97 | Identity migration uses fuzzy symbol lookup | 44 | 19 |
| 98 | A shadowed leaf erases the surrounding type structure | 9 | 10 |
| 99 | Associated-call arguments bypass body lexical scope | 5 | 5 |
| 100 | Value declaration categories are resolved separately | 5 | 0 |
| 101 | Complete compatibility, regression-gate and performance validation | — | 0 |

These are expected counts in the original 451-cell matrix. Add focused tests for
new helper contracts separately. Unexpected changes in passing or failing cells
require investigation, even if the total happens to match.

## Stage 97: Exact legacy identity migration

**Cause.** `src/data_flow/identity.rs` uses `CallGraph::find_function` for writes
and fallback reads. That method intentionally permits nearest-line and
cross-file matches. Those are unsuitable for moving stored facts between IDs.

**Change.** Add a narrowly named internal exact-or-unique-legacy lookup in the
call-graph layer and use it throughout the data-flow identity bridge. Reuse the
existing candidate bucket, then filter by exact file, qualified source name and
line. Do not normalize distinct source names into equality or select by line
distance. A known column must match exactly. A missing column may match only one
definition; an exact legacy node must not short-circuit a same-line tie.

Keep `find_function` and its broader consumers unchanged. Retain the direct
exact-key read/write fast path and current empty-graph behavior. When an incoming
record cannot be upgraded uniquely, keep its original storage identity; never
attach it to another definition. This avoids inventing a new rejection policy
for standalone records.

Use the same bridge for every fact family, both transformation endpoints and
I/O append migration. Audit the existing `identity_candidates` path as well as
`storage_identity`; fixing writes alone would leave a second fuzzy route.

**Small contract decision to cover before coding.** For legacy records, treat
an absent/empty `module_path` as missing metadata and permit only a unique exact
source match. When it is explicitly populated, require that it agree. Exact IDs
continue to use all their fields. Add tests for matching, missing and conflicting
metadata and for a legacy node coexisting with column-bearing same-line nodes.
This extends a dimension the baseline deliberately left unspecified.

**Acceptance.** All 338 identity cells pass, including current/legacy JSON,
sentinels, append operations and independently varied endpoints. Existing fuzzy
lookup tests and non-Rust identity consumers retain their behavior. No new
serialized fields, persistent index or full-graph scan per fact operation.

## Stage 98: Preserve type structure and reference constraints

**Cause.** `Body::declared_type` in `body.rs` replaces an entire type containing
any local shadow with `Uncertain(UnavailablePath)`. Even `&dyn T` loses its
reference shape. `expressions.rs::infer_at` then turns dereferencing that fact
into unconstrained `Unknown`. The six receiver and three propagation failures
are consequences of this shared loss.

**Change.** Introduce one body-scoped lowering path that preserves references,
tuples, nominal owners and generic argument positions. Mark the unsupported
shadowed leaf as unavailable, retaining its scope and uncertainty reason. For
dynamic bounds, use the existing unresolved-bound representation with the
original lexical context. Reuse existing workspace lowering for declaration
resolution rather than creating a second independent type interpreter.

Add a pure reference-projection operation that peels represented references and
preserves uncertainty and represented alternatives. Use it for explicit
dereference and the corresponding reference-pattern operation where applicable.
It must not implement custom `Deref`, strip arbitrary nominal wrappers or
replace a constrained unavailable fact with a wildcard. Unsupported operations
must remain distinguishable from supported reference projection.

**Scope boundary.** Lower type syntax written in the current body using that
body's bindings. Expand an alias body, field declaration or return signature in
its declaration context, substituting already-resolved caller argument facts.
Do not pass a caller's shadow-name set through all recursive workspace expansion.
That would break the returned-bound cases that currently pass.

**Acceptance.** The nine dereference/propagation failures become green. Add pure
tests for nested references, tuple/nominal argument leaves, projection of
uncertain alternatives, and declaration-context isolation. All primitive,
qualified-owner and returned-bound controls remain unchanged. The five generic
call failures may remain until Stage 99.

## Stage 99: Lower associated-call types once in their lexical scope

**Cause.** `expression_paths.rs::lookup_path` checks the path head and qself
owner, then `index/lookup.rs` reconstructs generic types using module scope.
Arguments in `W::<A>::hit()` and the trait part of `<Owner as T<A>>::hit()` bypass
the body shadow checks.

**Change.** Normalize supported associated-call forms into a small internal
query containing the already-lowered owner, optional trait identity and trait
arguments, member name, and explicit callable arguments. Both `W::<A>::hit()`
and `<W<A>>::hit()` must feed the same owner lookup. Qualified trait arguments
must use the Stage 98 lowering rules too.

Extract the existing candidate-filtering core to consume those facts. Public
AST helpers lower in their supplied context and call that same core; body
analysis supplies body-scoped facts. Keep ASTs and spans confined to their
active batch. Do not send a resolved fact back through name lookup or string
parsing, and do not add separate recursive shadow guards to each spelling.

Use the same query for edge selection and result-type propagation, including
constructor substitutions. Preserve lexical type/value distinctions at the
path head. Paths that cannot be classified within the existing resolver remain
conservative and carry their constraints.

**Acceptance.** All 74 constraint rows pass, including the five generic rows.
Add focused equivalence tests for normalized owner syntax and nested trait
arguments. The qualified generic-trait controls retain their current uncertain
candidate sets; expanded trait solving and resolving block-local aliases to
their actual targets are not required.

## Stage 100: Resolve complete value bindings before projecting outcomes

**Cause.** Namespace precedence in `index/paths.rs` is already shared, but its
consumers split the results. `lookup.rs` sees only free functions;
`index/values.rs` returns constants before constructors and recognizes only a
single constructor. A mixed binding set therefore looks unique or disappears.

**Change.** Add a lightweight internal value-binding result over the existing
index maps. It collects functions, constructor declarations and constants/statics
after lexical namespace precedence has been applied. Deduplicate by declaration
identity, not import path or resulting type. Retain unresolved explicit-binding
conflicts as ambiguity even when no indexed declaration represents them.

Derive these projections from that one result:

- Callable candidates and whether a resolved edge is justified. A lone function
  is still uncertain if another value binding competes with it.
- Value or invocation-result facts from all represented alternatives, preserving
  the union of admissible owners and the ambiguity reason.
- Constructor recognition, so constructor-only expressions produce neither
  callable nodes nor unavailable-function diagnostics.

Determine lexical ambiguity before checking constructor arity or invocation
shape. Those checks must not erase a competing binding. Two declarations remain
ambiguous even if their resulting types coincide; two imports of the same
declaration do not.

Update `expressions.rs` and `visitor.rs` so inference and recording consume the
same binding result and constructors' argument calls are visited once. Prefer
passing the result within a visit over adding a global mutable cache. Keep
existing local/explicit-over-glob precedence and type-only coexistence intact.

**Acceptance.** All 39 namespace rows pass. Both constructor owners survive
ambiguous globs without unrelated methods; mixed function/constructor bindings
remain uncertain; constant/constructor alternatives both survive. Add focused
tests for repeated imports, equal-type distinct declarations, unresolved
explicit conflicts, and calls inside ambiguous constructor arguments.

## Stage 101: Acceptance, maintained coverage and performance

Run every matrix after each stage with the documented Cargo `--no-fail-fast`
command. Require the intended failing group to close while every previous
positive stays green. Do not relax expectations to fit implementation behavior;
any semantic correction needs an independent justification.

When the repairs are green, register the six new integration targets in the
existing integration recipe/profile so routine integration validation executes
them. Use the existing Cargo/Nextest tooling; preserve the fast suite's scope
and document that `just test` alone is not the resolution acceptance gate.

Final validation includes:

- All original 451 cells, new helper-contract tests, 113 compiler classifications,
  five harness rejection tests and eight layout/order variants.
- Existing nine-gap/review regressions, enhancement lifecycle, direct/cached
  extraction, and 199/200/201/401-file workspaces with shuffled order and source
  snapshots. Full node/role/evidence/uncertainty parity and merge idempotence.
- Consumer checks: forbidden possible targets receive no dead-code protection;
  possible edges remain excluded from resolved counts and coverage.
- Legacy JSON, current Postcard, public JSON v3 and other-language contracts.
- `just fmt`, `CARGO_NET_OFFLINE=true just test`, the expanded integration gate,
  and `cargo clippy --offline --all-targets --all-features -- -D warnings`.

Benchmark the unchanged production baseline at `36d6f670` against the final
implementation using the existing Criterion `call_graph_bench` target in the
debug profile, including synthetic cross-batch cases. For full repository
measurements, reuse the documented CLI/context/LCOV command with `--no-tui` in
`docs/benchmarks/rust-resolution-nine-gaps.md`. Use identical frozen source and
LCOV inputs, one excluded warmup and five measured runs per revision. Record
graph build, history preload, function scoring, total time and peak RSS;
investigate median runtime increases above 10% or peak memory above 20%.
Criterion execution checks alone do not establish a repository performance
result. Do not introduce scripts, custom runners, dependencies or release builds.

## Delivery constraints

Keep repair commits grouped by exact identity, structural type preservation,
lexical call lowering and value-binding resolution, followed by validation/gate
documentation as appropriate. Update the implementation plan and the matrix
report with actual counts and measurements. The test-baseline commit is
intentionally red at the user's request; do not describe intermediate partial
repairs as full-suite completion.

Preserve public JSON v3, extraction entry points, owned workspace metadata,
bounded batches, source snapshots, Git-history negative caching, source names
and live scoring progress. No rust-analyzer integration, fixture Cargo builds,
general macros, full local declaration indexing, arbitrary generic inference,
expanded trait solving or fixed-point loop analysis. Older unversioned Postcard
migration remains outside scope.
