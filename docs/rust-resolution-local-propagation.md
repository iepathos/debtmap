# Rust local propagation repairs

A uniquely resolved tuple-struct constructor supplies its nominal result type and explicit generic substitutions. Constructor shape and arity are declaration metadata; constructors do not manufacture callable nodes or unavailable-function diagnostics. Argument calls retain their normal source sites. Generic parameters that require argument-based inference remain unknown, and explicit generic arguments respect block type shadows.

Conditions carry separate successful and unsuccessful binding states. The right side of `&&` sees successful left-side bindings, while `||` sees unsuccessful bindings. Then branches and while bodies receive successful bindings; outer facts are conservatively joined afterward. Loop writes are still invalidated, and each source call is visited once.

The lexical fixture target includes constructor fields, missing generic inference, constructor/type namespace coexistence, successful nested let chains, while let chains, conditional side effects and scope exits. Every fixture runs through direct, cached, sequential and parallel extraction; source-builder comparisons include complete graph metadata.

Validation: `cargo test --offline --test rust_method_resolution_lexical --test rust_method_resolution_scope --test rust_method_resolution_review_fixes` in debug mode. Arbitrary generic inference and fixed-point loop analysis remain deferred.
