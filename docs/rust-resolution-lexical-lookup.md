# Rust lexical lookup repairs

Rust call resolution resolves the first path segment in its type or value namespace. Local declarations and explicit imports take precedence over direct glob imports. Conflicting explicit bindings remain uncertain when an imported declaration is unavailable; available owner identities remain admissible candidates. Qualified paths and trait qualification retain this uncertainty.

Block items shadow only the namespaces they introduce. Functions, constants and statics shadow values; type aliases, enums, unions and traits shadow types. Tuple and unit structs also introduce constructor values. Established imports supply namespace membership; unavailable imports conservatively shadow both. These shadows apply throughout the block and preserve initializer-before-binding behavior. Loop write discovery uses the same namespace rules.

The permanent lexical fixtures run through direct AST extraction, cached sources, and sequential and parallel source-file builders. Assertions use source identities and exact possible-target sets. The source builders compare nodes, metrics, roles, role evidence, resolved calls, edge evidence, uncertainty and exclusions before rebasing fixture paths.

Validation: `cargo test --offline --test rust_method_resolution_lexical --test rust_method_resolution_scope --test rust_method_resolution_review_fixes` in debug mode. Full block-local declaration indexing and transitive glob reexports remain outside this repair.

Shadow checks also traverse qualified receiver types and trait bounds such as
`dyn T`. Unindexed block-local declarations retain unavailable receiver constraints,
so module-level namesakes cannot become resolved or possible targets. Explicit
`crate::` qualification continues to select the module-level declaration.
`tests/rust_method_resolution_review_followup.rs` covers these cases and controls
through direct, cached, enhanced, sequential, parallel and repeated-merge paths.
