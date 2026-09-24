# Rust workspace lookup performance

The first complete-workspace measurement exceeded the 10% repository runtime
investigation threshold. A sampling profile identified repeated namespace path
resolution, callable lookup, and workspace membership checks during body
analysis. The graph contained the same 9,201 definitions and 8,801 call sites in
the synthetic baseline and complete-workspace diagnostic runs.

The immutable workspace index now stores exact free-function paths and raw trait
declaration candidate positions for each callable. Exact free-function lookup
retains the previous basename restriction, workspace filtering, and callable
ordering. Trait candidates retain declaration positions, including ambiguous and
non-trait bindings, so caching does not expand trait solving or reinterpret
aliases.

Workspace membership is finalized once from source-established module roots.
Only a file with exactly one module context and exactly one root receives a
membership identifier. Cross-file checks compare those identifiers; same-file
checks remain valid even when membership is absent or ambiguous. A regression
compares the cached result against the previous predicate across missing,
ambiguous, and distinct module roots.

Cached-source graph merging also checks whether every source definition already
exists under its exact identity in the metric graph. That case merges directly,
retaining metric precedence and combined role evidence without rebuilding two
complete graph maps. Legacy or mismatched identities still use canonicalization.
A regression preserves source trait roles while giving base metrics precedence.

Import target filtering also avoids repeating the requested namespace lookup.
These changes reduce repeated work without changing resolution policy. Final
runtime and memory measurements are recorded in the
[repair validation report](benchmarks/rust-resolution-nine-gaps.md); the sampling
profile alone is not evidence of a measured speedup.
