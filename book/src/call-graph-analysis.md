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

Debtmap builds call graphs from AST analysis:

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

### Resolution Mechanisms

The call graph analyzer handles:

- **Trait method resolution** - Resolves trait method calls to implementations
- **Macro expansion tracking** - Tracks calls within macro expansions
- **Module path resolution** - Resolves fully-qualified paths
- **External crate filtering** - Optionally excludes standard library calls

## Configuration

Control call graph behavior with these options:

```toml
[call_graph]
# Enable parallel call graph construction
parallel = true

# Show external crate calls in output
show_external_calls = false

# Maximum callers/callees to display per function
max_display_edges = 10

# Enable call graph debugging output
debug = false
```

## Usage

### Basic Call Graph Analysis

```bash
# Analyze with call graph enabled (default)
debtmap analyze .

# Show caller/callee relationships
debtmap analyze . --show-dependencies

# Debug call graph construction
debtmap analyze . --debug-call-graph
```

### Filtering External Calls

```bash
# Hide standard library calls
debtmap analyze . --no-external-calls

# Show all calls including external crates
debtmap analyze . --show-external-calls
```

## Visualization

Call graph information appears in output formats:

```
Function: process_data
  Complexity: 12
  Called by: main, handle_request (2 callers)
  Calls: validate_input, parse_data, transform (3 callees)
```

## See Also

- [Architectural Analysis](architectural-analysis.md) - Circular dependency detection
- [Context Providers](context-providers.md) - Critical path analysis
- [Coverage Integration](coverage-integration.md) - Transitive coverage
