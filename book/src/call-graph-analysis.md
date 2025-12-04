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

Debtmap builds call graphs through a three-phase AST-based construction process:

1. **Extract functions and collect unresolved calls** - Parse each file to identify function definitions and call expressions
2. **Resolve calls using CallResolver and PathResolver** - Match call expressions to function definitions within the same file
3. **Final cross-file resolution** - Resolve remaining calls across module boundaries

This multi-phase approach ensures accurate resolution while handling complex scenarios like trait methods, macros, and module imports.

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

The call graph analyzer handles complex resolution scenarios:

- **Trait method resolution** - Resolves trait method calls to implementations using struct prefixes (e.g., `Processor::process`)
- **Macro expansion tracking** - Classifies and tracks calls within macros (collection, formatting, assertion, and logging macros)
- **Module path resolution** - Resolves fully-qualified paths across module boundaries
- **Cross-file resolution** - Matches unresolved calls (marked with line 0) to actual function definitions

**Source**: Resolution mechanisms from src/analyzers/call_graph/trait_handling.rs, src/analyzers/call_graph/macro_expansion.rs, src/analyzers/call_graph/path_resolver.rs

### Parallel Construction

Call graph construction runs in parallel by default for improved performance. You can disable parallel processing with `--no-parallel` for debugging purposes, though this affects overall analysis performance, not just call graph construction.

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

**Source**: Configuration fields from src/config/core.rs:149-167 (AnalysisSettings)

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

### Display Control Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--show-dependencies` | false | Show dependency information (callers/callees) in output |
| `--no-dependencies` | false | Hide dependency information (conflicts with --show-dependencies) |
| `--max-callers <N>` | 5 | Maximum number of callers to display per function |
| `--max-callees <N>` | 5 | Maximum number of callees to display per function |
| `--show-external-calls` | false | Show external crate calls in dependencies |
| `--show-std-lib-calls` | false | Show standard library calls in dependencies |

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

**Source**: CLI flags from src/cli.rs:163-289

## Usage

### Basic Call Graph Analysis

```bash
# Analyze with call graph enabled (default)
debtmap analyze .

# Show caller/callee relationships in output
debtmap analyze . --show-dependencies

# Limit displayed relationships
debtmap analyze . --show-dependencies --max-callers 3 --max-callees 3
```

### Filtering External Calls

By default, Debtmap filters both external crate calls and standard library calls for cleaner output. The call graph contains all edges; filtering only affects display output.

```bash
# Default: external and standard library calls are hidden
debtmap analyze .

# Show external crate calls (e.g., from dependencies)
debtmap analyze . --show-dependencies --show-external-calls

# Show standard library calls (std::, core::, alloc::)
debtmap analyze . --show-dependencies --show-std-lib-calls

# Show both external and standard library calls
debtmap analyze . --show-dependencies --show-external-calls --show-std-lib-calls
```

**Important**: External call filtering happens at display time, not during graph construction. This means `--debug-call-graph` may show more calls than regular output.

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

**Source**: Validation implementation from tests/call_graph_debug_output_test.rs:134-151

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

1. **Check filtering** - Use `--show-external-calls` and `--show-std-lib-calls` to see all edges
2. **Validate structure** - Run `--validate-call-graph` to check for structural issues
3. **Review debug output** - Use `--debug-call-graph` to see complete graph before filtering

## See Also

- [Architectural Analysis](architectural-analysis.md) - Circular dependency detection
- [Context Providers](context-providers.md) - Critical path analysis
- [Coverage Integration](coverage-integration.md) - Transitive coverage propagation
- [Configuration](configuration.md) - Complete configuration reference
- [CLI Reference](cli-reference.md) - All command-line flags
