# Rust receiver constraints

Dynamic and bounded generic receivers carry the identity of each trait declaration.
Bounds are resolved after the complete workspace index is available. Their original
file and lexical module are retained when a declaration is missing or ambiguous;
propagation through return types, aliases, fields, references, and generic
substitutions does not resolve those names again in the caller's module.

Primitive types have explicit receiver identities. A declaration or explicit import
with the same lexical type name is considered before recognizing an unqualified
primitive. Numeric literals with explicit suffixes and boolean, character, byte,
and string literals provide primitive facts. Unsuffixed numbers do not trigger
argument-based type inference.

A primitive receiver excludes incompatible nominal project implementations. Local
trait implementations for the same primitive remain admissible. Explicit trait
qualification can resolve a matching implementation, while ordinary dotted calls
remain uncertain because built-in method precedence is not modeled. Ambiguous
implementation-owner facts preserve each admissible owner without turning an
unknown owner into a match.

Possible calls remain separate from resolved edges and coverage propagation. The
receiver regression target verifies this distinction after trait enhancement and
repeated graph merges: an impossible nominal method remains eligible for dead-code
detection, and a possible primitive method receives neither a resolved caller count
nor indirect coverage from a covered caller.

Run `cargo test --offline --test rust_method_resolution_receiver_context` for the
six receiver regression cases, including direct AST, cached-source, sequential,
and parallel extraction and nested out-of-line module contexts. The cases use
source locations and complete function identities when comparing targets. They do
not establish general Rust precision or repository-scale performance.

This change does not add fields to public JSON v3. General trait solving, default
trait-body instantiation, arbitrary generic inference, and custom `Deref` remain
outside this repair.
