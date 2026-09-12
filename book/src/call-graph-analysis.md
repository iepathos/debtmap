# Call Graph Analysis

Debtmap constructs detailed call graphs to track function relationships and dependencies across your codebase. This enables critical path identification, circular dependency detection, and transitive coverage propagation.

## Overview

Call graph analysis builds a comprehensive map of which functions call which other functions. This information powers several key features:

- **Critical path identification** - Find frequently-called functions that deserve extra attention
- **Circular dependency detection** - Identify problematic circular call patterns
- **Transitive coverage** - Propagate test coverage through the call graph
- **Dependency visualization** - See caller/callee relationships in output
- **Risk assessment** - Factor calling patterns into priority scoring

## Call Graph Construction

For Rust, Debtmap builds an immutable declaration index over all discovered
source files before analyzing function bodies:

1. **Index declarations** — Record types, aliases, fields, imports, callable signatures,
   receiver forms, and definition locations.
2. **Collect source facts** — Track lexical bindings and propagate supported receiver
   types through expressions.
3. **Record outcomes** — Add an ordinary edge when declaration evidence justifies a
   target; otherwise preserve the call and its admissible possible targets separately.

The standard `analyze` command also uses this resolver when its graph is built from
cached extraction data. Rust extraction retains a source snapshot so that the complete
declaration index can be reconstructed before resolving bodies. Older cache records
without source snapshots preserve uncertain method calls instead of recovering edges
through name-only matching. Public JSON output does not expose the internal snapshot.
Rust extraction, metrics, and cached graphs use the same module-qualified function
names, including multiple inline modules on one source line. Current extraction
records support Postcard round trips with present or absent source snapshots; this
does not provide migration for older binary cache layouts.
Keeping these snapshots costs source-sized storage and one additional parse per Rust
file when constructing the workspace index; it avoids parsing separately for each
function. Resolution totals are available in existing debug logs with
`RUST_LOG=debtmap=debug`.

Git-history context retains qualified names as cache keys and searches the source
declaration spelling, such as `fn run` for `Worker::run`. A completed history preload
also remembers missing results, allowing cached file-history fallback without
repeating the repository scan during scoring. This history search remains based on
textual occurrences; it does not distinguish same-named historical declarations
within one file. Function-scoring progress advances as metrics finish.

Type identity includes its file, lexical module, and declaration location. Printed
names are display values. A dotted call cannot select a free function or an
associated function without a receiver. Known receiver constraints remain in force
through import lookup and fallback handling: `Timeline` never matches `PyTimeline`
merely because their names have a common suffix. A unique method name alone does
not establish a resolved edge.

```rust
// Example: Debtmap tracks these relationships
fn process_data(input: &str) -> Result<Data> {
    validate_input(input)?;  // Call edge: process_data -> validate_input
    parse_data(input)        // Call edge: process_data -> parse_data
}

fn validate_input(input: &str) -> Result<()> {
    // Call graph tracks this function as a callee
    Ok(())
}
```

**Source**: Example pattern from tests/call_graph_comprehensive_test.rs:48-94

### Resolution Mechanisms

The bounded Rust resolver supports:

- Parameters, `self`/`Self`, annotated and inferred locals, local aliases,
  references, dereferences of known references, and parentheses.
- Struct literals, declared unit structs, named and tuple fields, and known
  function/method return chains. Local async declarations propagate their output
  through `.await`.
- Explicit module paths and imports, including aliases, when the discovered source
  establishes one declaration identity. Module `#[path]` attributes normalize `.`
  and `..` for lookup while preserving the discovered file's identity. Absolute
  external paths such as `::std` do not select similarly named local modules.
- Constants and statics use their declared value types. Enums and type aliases
  do not establish unit-struct values merely by sharing a name.
- Direct generic substitution from explicit arguments and known receiver arguments
  into fields and return types. Lifetimes do not become part of a type name; const
  arguments are retained without evaluation. A declared `new() -> Other` returns
  `Other`, irrespective of its constructor-like name.
- Inherent methods and concrete trait implementations whose owner, trait scope,
  and requirements are established. Explicit trait qualification distinguishes
  competing implementation bodies from trait declarations and retains the enclosing
  impl's `Self` substitution.

Bindings follow lexical scope. An initializer sees the previous binding; an
unknown inner binding shadows an outer known binding. Unsupported pattern bindings
introduce unknown facts. Assignments update inferred types or preserve explicit
constraints with uncertainty; branch joins require agreement. Loops conservatively
invalidate inferred facts they modify.
Tuple annotations retain each component's declared owner when initializer facts
are unavailable. A contradictory component remains constrained uncertainty without
making unrelated owners possible or invalidating an agreeing sibling.

Project declarations named `clone`, `get`, or `any` receive the same receiver
checks as other methods. Their names alone do not establish library ownership.
Existing supported macro argument scanning, function-pointer tracking, and framework
registrations remain separate inputs to graph construction. Trait enhancement
consumes shared call uncertainty and cannot recreate ordinary method edges through
weaker name matching.

### Possible Calls and Dead Code

`CallGraph` retains uncertain calls with caller and call-site identity, lexical
module, query, available receiver information, sorted deduplicated candidate
identities, and a reason. Reasons distinguish unknown receivers, ambiguous
identities, unsupported type operations, unavailable definitions, and analysis
limits. Zero-candidate calls remain available for diagnostics. Repeated merges
preserve distinct sites and do not duplicate identical records.
Legacy extraction summaries without columns carry a separate occurrence ordinal,
so repeated calls on one line remain distinct without fabricated source positions.
Loading legacy graph records deduplicates edges and evidence and rebuilds indexes.
Sequential and parallel conversions also retain legacy entry-point and test flags.

Ordinary `get_callers` and `get_callees` queries return resolved relationships.
Possible targets do not increase caller/dependency counts or propagate coverage,
purity, or ordinary graph scores. Internal consumers can use `uncertain_calls`,
`get_possible_callers`, `get_possible_callees`, and
`get_transitive_possible_callees` to inspect uncertainty explicitly.

The caller-based dead-code classifier withholds `DeadCode` when an admissible
possible caller exists. Enhanced reachability analysis follows both resolved and
possible relations from its existing live roots, protecting reachable descendants.
An uncertain call in unreachable code does not become a new root. Definitely-live
queries retain their existing resolved semantics. Zero-candidate calls cannot
protect unrelated functions; missing project targets remain a limitation of
syntax-based dead-code analysis.

### Accuracy Limits

This is a bounded source resolver, not Rust's type checker. Dynamic dispatch and
unresolved generic trait receivers retain possible implementation targets.
Trait paths with generic or associated arguments and reference-owned trait impls
remain uncertain. Trait declarations without bodies are not graph nodes; default
trait bodies are not instantiated for concrete impls that omit an override. Nested
glob reexports are unsupported. An explicitly qualified or imported unavailable
receiver excludes unrelated project methods; a bare unavailable receiver may retain
a possible target found only by method name. Custom
`Deref`, blanket-impl solving, associated-type projection, arbitrary coercions,
general `?`/wrapper inference, and const-generic evaluation remain unsupported.
Block-local item declarations and imports are not indexed as independent lexical
module contexts; calls requiring those contexts remain uncertain. Generic arguments
are not inferred from arbitrary argument constraints. Alias and substitution
expansion stops at 32 levels; recursive aliases and other incomplete facts remain
uncertain instead of triggering a name-only guess.

Existing source discovery remains authoritative. The resolver does not invoke
Cargo project builds or metadata, download dependencies, execute build scripts, or
introduce general macro expansion. It does not use rust-analyzer. Ambiguous crate,
module, or configuration contexts may remain unresolved; filenames alone do not
establish cross-crate ownership.

These internal graph changes do not add fields to public JSON v3 output. Internal
serialized graphs in self-describing formats such as JSON preserve uncertainty and
evidence; older JSON graphs load with empty uncertainty. The existing graph serializer
does not support a postcard round trip, so binary-format compatibility is not claimed.
Caller counts and debt rankings can change as incorrect edges are removed or supported
calls are recovered.

The labeled regression corpus is in `tests/data/rust_method_resolution/`. Its
exact-edge precision and recall describe those fixtures only, not arbitrary Rust
projects. `scripts/benchmark_rust_method_resolution.py` measures five warmed debug
runs on staged inputs and verifies that all fixture files were analyzed. Timing
outputs are local evaluation artifacts.

### Parallel Construction

Sequential and parallel Rust builders use the same complete workspace resolution.
Parallel conversion preserves resolved edge evidence, node role evidence, and
uncertain calls. Call graph construction runs in parallel by default for improved performance. You can disable parallel processing with `--no-parallel` for debugging purposes, though this affects overall analysis performance, not just call graph construction.

## Configuration

Call graph behavior is controlled through two configuration sections:

### Analysis Settings

Configure advanced analysis features in the `[analysis]` section:

```toml
[analysis]
# Enable trait method resolution (default: depends on context)
enable_trait_analysis = true

# Enable function pointer and closure tracking (default: depends on context)
enable_function_pointer_tracking = true

# Enable framework pattern detection for tests and handlers (default: depends on context)
enable_framework_patterns = true

# Enable cross-module dependency analysis (default: depends on context)
enable_cross_module_analysis = true

# Maximum depth for transitive analysis (optional)
max_analysis_depth = 10
```

**Source**: Configuration fields from src/config/core.rs:159-175 (AnalysisSettings)

### Caller/Callee Display Settings

Configure how dependencies are displayed in the `[classification.caller_callee]` section:

```toml
[classification.caller_callee]
# Maximum number of callers to display per function (default: 5)
max_callers = 5

# Maximum number of callees to display per function (default: 5)
max_callees = 5

# Show external crate calls in dependencies (default: false)
show_external = false

# Show standard library calls in dependencies (default: false)
show_std_lib = false
```

**Source**: Configuration fields from src/config/classification.rs:5-50 (CallerCalleeConfig)

## CLI Reference

### Analysis Control Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--no-parallel` | false | Disable parallel processing (enabled by default) |

### Debug and Validation Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--debug-call-graph` | false | Enable detailed call graph debugging output |
| `--validate-call-graph` | false | Validate call graph structure and report issues |
| `--call-graph-stats` | false | Show call graph statistics with resolution percentiles (p50, p95, p99) |
| `--trace-function <NAMES>` | none | Trace specific functions during call resolution (comma-separated) |
| `--debug-format <FORMAT>` | text | Debug output format (text or json) |

**Source**: CLI flags from src/cli/args.rs:341-350

## Usage

### Basic Call Graph Analysis

```bash
# Analyze with call graph enabled (default)
debtmap analyze .

# Show call graph statistics
debtmap analyze . --call-graph-stats
```

### Inspecting Call Resolution

Use `--debug-call-graph` for detailed resolver output and `--trace-function <NAME>` to follow specific functions through call resolution.

**Source**: Filtering logic from src/priority/formatter/dependencies.rs:filter_dependencies

### Debugging Call Resolution

```bash
# Enable detailed call graph debugging
debtmap analyze . --debug-call-graph

# Trace specific functions during resolution
debtmap analyze . --trace-function "process_data,validate_input"

# Show call graph statistics with percentiles
debtmap analyze . --call-graph-stats

# Validate call graph structure
debtmap analyze . --validate-call-graph

# Disable parallel processing for debugging
debtmap analyze . --no-parallel
```

### Debug Output Format

Debug output includes:
- **Resolution statistics** - Success rates with percentiles (p50, p95, p99)
- **Timing information** - Performance metrics for each resolution phase
- **Function tracing** - Detailed resolution attempts for specified functions
- **Unresolved calls** - Calls that couldn't be matched to definitions

Macro expansion statistics show classification breakdown (collection macros, formatting macros, assertion macros, logging macros).

**Source**: Debug capabilities from src/analyzers/call_graph/debug.rs (DebugConfig, ResolutionStatistics)

## Visualization

Call graph information appears in output using Unicode tree-style rendering:

```
├─ DEPENDENCIES:
│  ├─ Called by (2):
│  │     * main
│  │     * handle_request
│  │     ... (showing 2 of 2)
│  ├─ Calls (3):
│       * validate_input
│       * parse_data
│       * transform
│       ... (showing 3 of 5)
```

**Source**: Tree-style rendering from src/priority/formatter/sections.rs:240-329

### Path Simplification

Long paths are simplified for readability:
- Short names: unchanged (e.g., `my_function`)
- Two-segment paths: unchanged (e.g., `helper::read_file`)
- Long paths: simplified to last two segments (e.g., `crate::utils::io::helper::read_file` → `helper::read_file`)

### Empty States

- **No callers**: "Called by: No direct callers detected"
- **No callees**: "Calls: Calls no other functions"

### Standard Library Detection

Standard library calls are filtered by default and include:
- Functions starting with `std::`, `core::`, or `alloc::`
- Common macros: `println`, `print`, `eprintln`, `eprint`, `write`, `writeln`, `format`, `panic`, `assert`, `debug_assert`

External crate calls are identified as functions containing `::` that aren't in the standard library or the current crate (`crate::`).

**Source**: Detection logic from src/priority/formatter/dependencies.rs:is_standard_library_call, is_external_crate_call

## Validation and Health Scoring

The call graph validator checks for structural issues:

```bash
debtmap analyze . --validate-call-graph
```

Validation reports include:
- **Health score** - Overall graph quality (0-100)
- **Structural issues** - Orphaned functions, disconnected components
- **Warnings** - Potential resolution problems

**Source**: Validation implementation from src/analyzers/call_graph/validation.rs:185 (CallGraphValidator)

## Performance Tuning

For large codebases, consider these performance optimizations:

- **Disable parallel processing** (`--no-parallel`) - Only for debugging; reduces performance
- **Control analysis depth** - Use `max_analysis_depth` in configuration to limit transitive analysis
- **Disable optional analysis** - Turn off `enable_trait_analysis`, `enable_function_pointer_tracking`, or `enable_framework_patterns` if not needed

## Troubleshooting

### Unresolved Calls

If you see unresolved calls in debug output:

1. **Check imports** - Ensure all modules are properly imported
2. **Verify visibility** - Confirm functions are accessible (not private across module boundaries)
3. **Review module structure** - Complex module hierarchies may require explicit path configuration
4. **Use tracing** - Run with `--trace-function` to see detailed resolution attempts

### Incorrect Caller/Callee Counts

If counts seem wrong:

1. **Validate structure** - Run `--validate-call-graph` to check for structural issues
2. **Review debug output** - Use `--debug-call-graph` to see call resolution details

## See Also

- [Architectural Analysis](architectural-analysis.md) - Circular dependency detection
- [Context Providers](context-providers.md) - Critical path analysis
- [Coverage Integration](coverage-integration.md) - Transitive coverage propagation
- [Configuration](configuration.md) - Complete configuration reference
- [CLI Reference](cli-reference.md) - All command-line flags
