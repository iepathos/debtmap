# Debtmap Architecture

## Scope

Debtmap is a multi-language static analyzer for Rust, Python, JavaScript, TypeScript, Go,
and Solidity. Language support is intentionally not uniform: each adapter reports only the
evidence it can establish, and unresolved relationships should remain absent rather than being
guessed.

This document describes the current production path. Planned work and migration status live in
the local `IMPLEMENTATION_PLAN.md` development artifact, which is intentionally not versioned.

## Production Analysis Path

The supported project-analysis entry point is the `debtmap analyze` command. Its orchestration is
implemented under `src/commands/analyze/` and delegates to the canonical unified-analysis builder
in `src/builders/unified_analysis.rs`.

```text
CLI and effective configuration
        |
        v
file discovery and language detection
        |
        v
language-specific parsing and metric extraction
        |
        v
call graph, purity, coverage, and optional context evidence
        |
        v
debt detection, scoring, filtering, and aggregation
        |
        v
terminal, Markdown, JSON, DOT, or TUI output
```

The generic builder in `src/pipeline/` is usable for composing custom typed stages. Its legacy
analysis presets are incomplete, deprecated, and fail closed at execution. They are not an
alternative project-analysis API.

## Layers And Boundaries

### CLI and orchestration

- `src/cli/` defines command-line arguments and converts them into typed command inputs.
- `src/commands/analyze/` coordinates discovery, analysis, filtering, and output.
- `src/config/` loads project configuration. CLI overrides and configuration must eventually be
  normalized into one immutable analysis policy; that consolidation is tracked in Stage 41.

### Discovery and language adapters

- `src/io/walker.rs` discovers candidate source files.
- `src/core/mod.rs` contains the canonical six-language enumeration.
- `src/analyzers/` contains language adapters and shared analyzer traits.
- Rust uses `syn`; Python, JavaScript, TypeScript, Go, and Solidity use tree-sitter based adapters.

Adapters own syntax-specific parsing. Shared phases should consume explicit facts from adapters
instead of reproducing language-name or path heuristics.

### Shared analysis

- `src/builders/unified_analysis.rs` is the canonical analysis coordinator.
- `src/analyzers/call_graph/` builds and resolves call relationships.
- `src/analysis/` contains cross-file and multi-pass algorithms.
- `src/complexity/`, `src/debt/`, and `src/risk/` calculate focused evidence.
- `src/priority/` combines evidence into ranked function and file findings.

Sequential and Rayon-backed execution are expected to use the same pure scoring and finalization
logic. Remaining parity and performance work is tracked in Stage 42; callers should not treat the
generic pipeline presets as a second implementation.

### Output boundary

- `src/output/` owns canonical terminal, Markdown, JSON, and DOT conversion.
- `src/io/writers/` contains writer implementations.
- `src/tui/` presents the same analysis interactively.

Function findings do not currently receive generated refactoring advice. Human-readable output
therefore labels them as findings and omits action or rationale sections when no computed guidance
exists. File-level and specialized analyzers may still emit guidance when they actually compute it.

The canonical CLI JSON contract is v4, defined in
[`schemas/debtmap-output-v4.schema.json`](schemas/debtmap-output-v4.schema.json). The v3 contract
remains frozen for compatibility. Consumers must check `format_version`; provenance was added in a
new envelope rather than mutating v3.

## Evidence Availability

Static metrics and syntax evidence come from analyzed source files. Other signals are optional:

- Coverage affects risk only when an LCOV file is supplied and successfully loaded.
- Git history and other context providers run only when context is requested and available.
- A single report has no historical trend baseline. Snapshot reports must say that trend is
  unavailable rather than inferring stability.
- Missing or ambiguous call resolution is not positive evidence of no callers or no side effects.

Output should distinguish a requested input from evidence successfully loaded and should surface
partial-analysis state. The versioned receipt needed for that machine-readable contract is tracked
in Stage 53.

## Data And Error Ownership

The intended dependency direction is inward:

```text
I/O shell -> typed orchestration -> pure analysis transforms -> core data types
```

Fallible boundaries return contextual errors. Analysis code should not silently turn parse,
resolution, or file failures into authoritative empty results. Compatibility error modules still
exist in this pre-1.0 crate; `src/debtmap_error.rs` is the target facade for future consolidation.

## Determinism And Parallelism

Rayon is used for independent file and scoring work. Parallelism is a scheduling concern, not a
separate analysis policy. Given the same source snapshot, effective policy, reference time, and
optional evidence, normalized findings and scores should agree between sequential and parallel
execution. Golden parity tests guard the portions already consolidated; Stage 42 tracks the
remaining kernel and profiling work.

## Testing And Release Gates

- Unit tests live beside pure analysis functions.
- Cross-module and CLI contracts live under `tests/`.
- JSON schemas and canonical fixtures live under `schemas/` and `tests/fixtures/output/`.
- Criterion benchmarks live under `benches/`.
- Required local verification uses debug builds: `just fmt` and `just test`.

Before release, public output, pipeline failure behavior, schema compatibility, doctests, clippy,
the full non-ignored debug suite, and a debug self-analysis should all pass.
