# Advanced Options

This subsection covers advanced configuration options in Debtmap, including entropy analysis, god object detection, context-aware false positive reduction, and parallel processing.

## Overview

Debtmap provides several advanced analysis features that can be tuned for specific project needs:

- **Entropy Analysis** - Information theory-based complexity dampening
- **God Object Detection** - Detection of overly complex types and modules
- **Context-Aware Detection** - Smart false positive reduction based on code context
- **Parallel Processing** - Multi-threaded analysis for large codebases

## Entropy Analysis

Entropy analysis uses information theory to identify repetitive code patterns that inflate complexity metrics. When code has low entropy (highly repetitive), its complexity score is dampened to reflect its true cognitive load.

**Source:** `src/config/languages.rs:65-127` (`EntropyConfig`)

### Configuration

Configure entropy analysis in the `[entropy]` section of `.debtmap.toml`:

```toml
[entropy]
enabled = true                    # Enable entropy-based scoring (default: true)
weight = 1.0                      # Weight of entropy in adjustment (0.0-1.0, default: 1.0)
min_tokens = 20                   # Minimum tokens for calculation (default: 20)
pattern_threshold = 0.7           # Pattern similarity threshold (0.0-1.0, default: 0.7)
entropy_threshold = 0.4           # Low entropy detection threshold (default: 0.4)

# Branch analysis
branch_threshold = 0.8            # Branch similarity threshold (default: 0.8)

# Reduction caps
max_repetition_reduction = 0.20   # Max 20% reduction for repetition (default: 0.20)
max_entropy_reduction = 0.15      # Max 15% reduction for low entropy (default: 0.15)
max_branch_reduction = 0.25       # Max 25% reduction for similar branches (default: 0.25)
max_combined_reduction = 0.30     # Max 30% total reduction cap (default: 0.30)
```

### Configuration Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | true | Enable entropy-based scoring |
| `weight` | `f64` | 1.0 | Weight of entropy in complexity adjustment |
| `min_tokens` | `usize` | 20 | Minimum tokens required for calculation |
| `pattern_threshold` | `f64` | 0.7 | Threshold for pattern repetition detection |
| `entropy_threshold` | `f64` | 0.4 | Threshold for low entropy detection |
| `branch_threshold` | `f64` | 0.8 | Threshold for branch similarity detection |
| `max_repetition_reduction` | `f64` | 0.20 | Maximum reduction for high repetition |
| `max_entropy_reduction` | `f64` | 0.15 | Maximum reduction for low entropy |
| `max_branch_reduction` | `f64` | 0.25 | Maximum reduction for similar branches |
| `max_combined_reduction` | `f64` | 0.30 | Maximum combined reduction cap |

### How Entropy Dampening Works

**Source:** `src/complexity/entropy_core.rs:19-48` (`EntropyAnalysis`)

Entropy analysis calculates several metrics for each function:

1. **Token Entropy** (`entropy_score`) - Shannon entropy of code tokens (0.0-1.0)
   - High entropy (>0.4): Unique, varied code patterns
   - Low entropy (<0.4): Repetitive patterns, triggers dampening

2. **Pattern Repetition** (`pattern_repetition`) - How much code repeats (0.0-1.0)
   - High values indicate repeated code blocks

3. **Branch Similarity** (`branch_similarity`) - Similarity between conditional branches
   - High values indicate similar match/if-else arms

4. **Dampening Factor** - Applied to complexity (0.5-1.0)
   - 1.0 = no dampening (genuine complexity)
   - 0.5 = maximum dampening (very repetitive code)

**Example Impact:**
```
Function: format_match_arms (20 cyclomatic complexity)
  Token Entropy: 0.3 (low - repetitive formatting)
  Pattern Repetition: 0.8 (high - repeated patterns)
  Dampening Factor: 0.7
  Adjusted Complexity: 14 (20 × 0.7)
```

### Use Cases

**Reduce false positives from match statements:**
```toml
[entropy]
enabled = true
pattern_threshold = 0.6    # More aggressive pattern detection
max_branch_reduction = 0.30 # Allow higher reduction for similar branches
```

**Strict analysis (minimal dampening):**
```toml
[entropy]
enabled = true
max_combined_reduction = 0.15  # Cap total reduction at 15%
```

**Disable entropy analysis:**
```toml
[entropy]
enabled = false
```

## God Object Detection

God object detection identifies types and files that have grown too large, accumulating too many responsibilities. Debtmap detects three types:

**Source:** `src/organization/god_object/core_types.rs:12-46` (`DetectionType`)

- **GodClass** - A single struct with >20 impl methods across multiple responsibilities
- **GodFile** - A file with >50 standalone functions and no struct definitions
- **GodModule** - A hybrid with both structs AND many standalone functions

### Detection Thresholds

**Source:** `src/organization/god_object/thresholds.rs:63-80` (`GodObjectThresholds`)

Default thresholds for detection:

| Threshold | Default | Description |
|-----------|---------|-------------|
| `max_methods` | 20 | Maximum methods before flagging as GodClass |
| `max_fields` | 15 | Maximum fields before flagging |
| `max_traits` | 5 | Maximum trait implementations |
| `max_lines` | 1000 | Maximum lines of code |
| `max_complexity` | 200 | Maximum total complexity |

**Fallback heuristics** for non-Rust files (`src/organization/god_object/heuristics.rs:20-22`):

| Threshold | Value | Description |
|-----------|-------|-------------|
| `HEURISTIC_MAX_FUNCTIONS` | 50 | Maximum functions in a file |
| `HEURISTIC_MAX_LINES` | 2000 | Maximum lines for heuristic detection |
| `HEURISTIC_MAX_FIELDS` | 30 | Maximum fields for heuristic detection |

### Language-Specific Thresholds

**Source:** `src/organization/god_object/thresholds.rs:84-102`

Debtmap provides language-specific thresholds:

**Rust (default):**
```
max_methods: 20, max_fields: 15, max_traits: 5
max_lines: 1000, max_complexity: 200
```

**Python (stricter):**
```
max_methods: 15, max_fields: 10, max_traits: 3
max_lines: 500, max_complexity: 150
```

### God Object Score Calculation

The `god_object_score` is calculated using a weighted algorithm that considers:

1. Method count relative to threshold
2. Field count relative to threshold
3. Number of distinct responsibilities
4. Lines of code
5. Average complexity per method

A higher score indicates a more severe god object problem. Scores are used to prioritize which types/files to refactor first.

### Viewing God Object Analysis

```bash
# Show detailed god object information
debtmap analyze . --show-god-objects

# Include split recommendations
debtmap analyze . --show-god-objects --verbose
```

## Context-Aware Detection

Context-aware detection reduces false positives by adjusting severity based on code context. Test files, example code, and debug functions receive different treatment than production code.

**Source:** `src/analyzers/context_aware.rs:18-35` (`ContextAwareAnalyzer`)

### Enabling Context-Aware Detection

Context-aware detection is enabled by default. To disable it:

```bash
# Disable context-aware detection
debtmap analyze . --no-context-aware

# Or via environment variable
DEBTMAP_CONTEXT_AWARE=false debtmap analyze .
```

### How Context Detection Works

**Source:** `src/cli/setup.rs:56-61`

When context-aware detection is enabled:

1. **File Type Detection** - Identifies test files, examples, benchmarks
2. **Function Context Analysis** - Detects function roles (entry point, debug, etc.)
3. **Rule-Based Adjustment** - Applies severity adjustments based on context

**Context Actions:**
- `Allow`/`Skip` - Remove the debt item entirely
- `Warn` - Reduce severity by 2 levels
- `ReduceSeverity(n)` - Reduce severity by n levels
- `Deny` - Keep the item unchanged

### Rule Actions

**Source:** `src/analyzers/context_aware.rs:50-63` (`process_rule_action`)

| Action | Effect | Example Use |
|--------|--------|-------------|
| `Allow` | Filters out item | Ignore TODOs in test files |
| `Skip` | Filters out item | Skip complexity in examples |
| `Warn` | Reduces severity by 2 | Flag but deprioritize |
| `ReduceSeverity(n)` | Reduces severity by n | Custom adjustment |
| `Deny` | No change | Keep full severity |

### Use Cases

**Analyze only production code (strict mode):**
```bash
# Disable context awareness - analyze everything equally
debtmap analyze . --no-context-aware
```

**Default behavior (recommended):**
```bash
# Context-aware is enabled by default
debtmap analyze .
# Test files, examples get reduced severity
```

## Parallel Processing

Parallel processing enables multi-threaded analysis for faster results on large codebases.

**Source:** `src/config/parallel.rs:36-57` (`ParallelConfig`)

### Configuration

Configure parallel processing in `.debtmap.toml`:

```toml
[parallel]
enabled = true           # Enable parallel processing (default: true)
max_concurrency = 8      # Maximum concurrent operations (default: num_cpus)
batch_size = 100         # Files per batch (default: 100)
```

### Configuration Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | true | Enable parallel processing |
| `max_concurrency` | `Option<usize>` | None (all cores) | Maximum concurrent operations |
| `batch_size` | `Option<usize>` | 100 | Batch size for chunked processing |

### CLI Flags

```bash
# Disable parallel processing (sequential mode)
debtmap analyze . --no-parallel

# Set specific number of worker threads
debtmap analyze . --jobs 4

# Use all available cores (default behavior)
debtmap analyze . --jobs 0
```

**Source:** `src/cli/args.rs:461-464` (--jobs flag)

### Batch Analysis Configuration

**Source:** `src/config/parallel.rs:125-143` (`BatchAnalysisConfig`)

For advanced batch processing control:

```toml
[batch_analysis]
fail_fast = false        # Stop at first error (default: false)
collect_timing = false   # Track analysis duration (default: false)

[batch_analysis.parallelism]
enabled = true
max_concurrency = 4
batch_size = 50
```

### Configuration Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fail_fast` | `bool` | false | Stop on first error vs accumulate all |
| `collect_timing` | `bool` | false | Track per-file analysis duration |
| `parallelism` | `ParallelConfig` | default | Nested parallelism settings |

### Performance Considerations

**When to use parallel processing (default):**
- Large codebases (>100 files)
- Multi-core systems
- CI/CD pipelines

**When to disable parallel processing:**
- Debugging analysis issues
- Memory-constrained environments
- Reproducible/deterministic output needed

```bash
# Sequential mode for debugging
debtmap analyze . --no-parallel

# Limited concurrency for memory constraints
debtmap analyze . --jobs 2
```

### Batch Processing Modes

**Source:** `src/config/parallel.rs:145-174` (`BatchAnalysisConfig` methods)

```rust
// Accumulating mode - collect all errors (default)
BatchAnalysisConfig::accumulating()

// Fail-fast mode - stop at first error
BatchAnalysisConfig::fail_fast()

// With timing collection for profiling
BatchAnalysisConfig::default().with_timing()

// Sequential processing for debugging
BatchAnalysisConfig::default().sequential()
```

## Environment Variables

Several advanced options can be controlled via environment variables:

| Variable | Effect | Example |
|----------|--------|---------|
| `DEBTMAP_CONTEXT_AWARE` | Enable/disable context-aware detection | `DEBTMAP_CONTEXT_AWARE=false` |
| `DEBTMAP_JOBS` | Set worker thread count | `DEBTMAP_JOBS=4` |

## Related Topics

- [Thresholds Configuration](thresholds.md) - Configure detection thresholds
- [Scoring Configuration](scoring.md) - Configure scoring weights
- [Parallel Processing](../parallel-processing.md) - Detailed parallel processing guide
- [Entropy Analysis](../entropy-analysis.md) - In-depth entropy analysis documentation
- [God Object Detection](../god-object-detection.md) - Detailed god object detection guide
