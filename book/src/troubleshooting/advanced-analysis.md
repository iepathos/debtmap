# Advanced Analysis Issues

Advanced troubleshooting for call graph, pattern detection, functional analysis, and other complex analysis features.

## Multi-Pass Analysis

Multi-pass analysis is enabled by default and performs two iterations to distinguish logical complexity from formatting artifacts.

```bash
# Multi-pass analysis runs by default
debtmap analyze .

# Disable for performance-critical scenarios
debtmap analyze . --no-multi-pass
```

**When to disable (`--no-multi-pass`)**:
- Performance-critical CI/CD pipelines
- Very large codebases (>100k LOC)
- Quick complexity checks during development

## Call Graph Debugging

**Available Flags**:

```bash
# Enable call graph debug output
debtmap analyze . --debug-call-graph

# Trace specific functions through call graph
debtmap analyze . --trace-function "function_name,another_function"

# Show only call graph statistics
debtmap analyze . --call-graph-stats

# Validate call graph consistency
debtmap analyze . --validate-call-graph
```

**Dependency Control Flags**:

```bash
# Show dependency information in results
debtmap analyze . --show-dependencies

# Limit number of callers shown per function
debtmap analyze . --max-callers 10

# Include external crate calls in call graph
debtmap analyze . --show-external-calls

# Include standard library calls
debtmap analyze . --show-std-lib-calls
```

## God Object Detection

**Flag**: `--no-god-object`

Disables god object (large class/module) detection.

**God Object Types**:
- **god_class**: Files with excessive complexity excluding test functions
- **god_file**: Files with excessive complexity including all functions
- **god_module**: Alias for god_file

```bash
# Disable god object detection entirely
debtmap analyze . --no-god-object

# See god object analysis with responsibility metrics
debtmap analyze . -vv
```

## Pattern Detection Issues

**Control Pattern Detection**:

```bash
# Disable pattern detection entirely
debtmap analyze . --no-pattern-detection

# Specify specific patterns to detect
debtmap analyze . --patterns "god_object,long_function,complex_conditional"

# Adjust pattern detection sensitivity (default: 0.7)
debtmap analyze . --pattern-threshold 0.6

# Show pattern detection warnings
debtmap analyze . --show-pattern-warnings
```

**Detected Patterns**:

*Debt patterns* (src/cli/args.rs:84):
- `god_object`: Classes/modules with too many responsibilities
- `long_function`: Functions exceeding length thresholds
- `complex_conditional`: Nested or complex branching logic
- `deep_nesting`: Excessive indentation depth

*Design patterns* (src/cli/args.rs:272-273):
- `observer`: Event-driven observer pattern implementations
- `singleton`: Singleton pattern usages
- `factory`: Factory pattern implementations
- `strategy`: Strategy pattern for interchangeable algorithms
- `callback`: Callback-based async patterns
- `template_method`: Template method pattern implementations

## Functional Analysis Issues

**Enable Functional Analysis**:

```bash
# Enable AST-based functional analysis
debtmap analyze . --ast-functional-analysis

# Use different strictness profiles
debtmap analyze . --ast-functional-analysis --functional-analysis-profile strict
debtmap analyze . --ast-functional-analysis --functional-analysis-profile balanced
debtmap analyze . --ast-functional-analysis --functional-analysis-profile lenient
```

**Common Issues**:

Too many false positives for legitimate imperative code:
```bash
# Use lenient profile
debtmap analyze . --ast-functional-analysis --functional-analysis-profile lenient
```

## Public API Detection Issues

**Control Public API Detection**:

```bash
# Disable public API detection
debtmap analyze . --no-public-api-detection

# Adjust public API detection threshold (default: 0.7)
debtmap analyze . --public-api-threshold 0.5
```

## Attribution Issues

Attribution analysis tracks where complexity originates in your code, separating logical complexity from formatting artifacts.

**Enable Attribution** (src/cli/args.rs:206-208):

```bash
# Show complexity attribution details
debtmap analyze . --attribution
```

**Understanding Attribution Output**:

Attribution breaks complexity into three categories:

- **Logical Complexity** (confidence: ~0.9): Genuine control flow and decision points
- **Formatting Artifacts** (confidence: ~0.75): Complexity from code formatting style
- **Pattern Complexity**: Complexity from recognized patterns

**Common Issues**:

*Attribution shows high formatting artifacts*:
```bash
# Use multi-pass analysis (enabled by default) to filter formatting
debtmap analyze .

# If disabled, re-enable it
debtmap analyze . --no-multi-pass  # DON'T do this
```

*Attribution confidence is too low*:
- Low confidence indicates the analysis couldn't reliably determine complexity sources
- This often happens with heavily macro-generated code or unusual control flow patterns

*Missing source mappings*:
- Source mappings require AST-level analysis
- Some dynamic patterns may not map to specific source locations

## See Also

- [Quick Fixes](quick-fixes.md) - Common problems with immediate solutions
- [Context Provider Issues](context-providers.md) - Provider-specific troubleshooting
- [Debug Mode](debug-mode.md) - Verbosity levels and diagnostics
