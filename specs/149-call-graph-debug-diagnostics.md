---
number: 149
title: Call Graph Debug and Diagnostics
category: testing
priority: medium
status: draft
dependencies: [146, 148]
created: 2025-10-24
---

# Specification 149: Call Graph Debug and Diagnostics

**Category**: testing
**Priority**: medium
**Status**: draft
**Dependencies**: Spec 146 (Cross-Module Call Resolution), Spec 148 (Enhanced FunctionId Matching)

## Context

Debugging call graph resolution issues is currently difficult because:
- No visibility into which calls failed to resolve and why
- No statistics on resolution success rates
- No way to trace resolution process for specific functions
- No validation that call graph data matches expected patterns

When users report "0 callers" issues, we have limited tools to diagnose whether:
- The call wasn't detected during AST parsing
- The call was detected but failed resolution
- The resolution succeeded but lookups are using wrong keys
- The data is correct but output formatting is wrong

**Current Debugging Process:**
1. Add manual println!() statements
2. Rebuild project
3. Run on test case
4. Remove println!() statements
5. Repeat

This is inefficient and doesn't provide systematic insights.

## Objective

Implement comprehensive debug and diagnostic tools for the call graph system, enabling developers and users to understand, validate, and troubleshoot call resolution issues effectively.

## Requirements

### Functional Requirements

1. **Debug Output Flag**
   - Add `--debug-call-graph` CLI flag
   - Show detailed resolution process
   - Display unresolved calls with failure reasons
   - Output statistics on resolution success rates

2. **Resolution Tracing**
   - Trace resolution attempts for specific functions
   - Show which strategies were tried and why they failed
   - Display candidate functions and disambiguation logic
   - Output confidence scores for matches

3. **Call Graph Validation**
   - Validate graph structure (no dangling edges)
   - Check for orphaned functions (no callers, no callees)
   - Identify suspicious patterns (e.g., all functions have 0 callers)
   - Report inconsistencies between call graph and source code

4. **Statistics and Reporting**
   - Resolution success rate (% of calls resolved)
   - Breakdown by resolution strategy (exact, fuzzy, name-only)
   - Distribution of caller/callee counts
   - Top unresolved call patterns

### Non-Functional Requirements

1. **Performance**: Debug mode should add < 20% overhead
2. **Usability**: Output should be readable and actionable
3. **Maintainability**: Easy to add new diagnostics
4. **Configurability**: Control verbosity and output format

## Acceptance Criteria

- [ ] `--debug-call-graph` flag shows detailed resolution process
- [ ] Unresolved calls are listed with failure reasons
- [ ] Resolution statistics include success rate and strategy breakdown
- [ ] Validation reports graph inconsistencies
- [ ] Tracing specific functions with `--trace-function` flag
- [ ] JSON output format for programmatic analysis
- [ ] Debug output is readable and actionable
- [ ] Performance overhead < 20% in debug mode
- [ ] Integration tests validate debug output format
- [ ] Documentation explains how to use debug tools

## Technical Details

### Implementation Approach

1. **Phase 1: Debug Infrastructure**
   - Add `CallGraphDebugger` struct to track resolution attempts
   - Implement `ResolutionAttempt` recording
   - Add statistics collection during resolution
   - Create debug output formatters

2. **Phase 2: CLI Integration**
   - Add `--debug-call-graph` flag to CLI
   - Add `--trace-function <name>` flag for specific functions
   - Add `--call-graph-stats` for summary statistics only
   - Support JSON output: `--debug-call-graph-format json`

3. **Phase 3: Validation Tools**
   - Implement graph structure validation
   - Add heuristic checks for common issues
   - Create validation report formatter
   - Integrate into test suite

### Architecture Changes

**File**: `src/analyzers/call_graph/debug.rs` (new)
- Add `CallGraphDebugger` struct
- Implement `ResolutionAttempt` recording
- Add statistics collection
- Create debug formatters

**File**: `src/analyzers/call_graph/call_resolution.rs`
- Add debug hooks to resolution process
- Record attempts when debugger is present
- Track failure reasons

**File**: `src/commands/analyze.rs`
- Add `--debug-call-graph` flag
- Create and attach debugger when flag present
- Output debug information after analysis

**File**: `src/analyzers/call_graph/validation.rs` (new)
- Implement graph validation checks
- Add heuristic-based issue detection
- Create validation report

### Data Structures

```rust
/// Debug information collector for call graph resolution
pub struct CallGraphDebugger {
    /// All resolution attempts (successful and failed)
    attempts: Vec<ResolutionAttempt>,

    /// Functions to trace (if --trace-function specified)
    trace_functions: HashSet<String>,

    /// Statistics
    stats: ResolutionStatistics,

    /// Configuration
    config: DebugConfig,
}

/// Record of a single resolution attempt
#[derive(Debug, Clone)]
pub struct ResolutionAttempt {
    /// The unresolved call being processed
    pub call: UnresolvedCall,

    /// Resolution strategy attempts in order
    pub strategy_attempts: Vec<StrategyAttempt>,

    /// Final result (None if unresolved)
    pub result: Option<FunctionId>,

    /// Total time spent on resolution
    pub duration: Duration,
}

/// Single strategy attempt details
#[derive(Debug, Clone)]
pub struct StrategyAttempt {
    /// Which strategy was tried
    pub strategy: ResolutionStrategy,

    /// Candidates found by this strategy
    pub candidates: Vec<FunctionId>,

    /// Why this attempt failed (if it did)
    pub failure_reason: Option<FailureReason>,

    /// Confidence score if successful
    pub confidence: Option<f32>,
}

/// Why a resolution attempt failed
#[derive(Debug, Clone)]
pub enum FailureReason {
    /// No candidates found
    NoCandidates,

    /// Multiple ambiguous candidates
    Ambiguous(Vec<FunctionId>),

    /// Candidates excluded by filters
    FilteredOut(String),

    /// Strategy not applicable
    NotApplicable,
}

/// Statistics collected during resolution
#[derive(Debug, Clone, Default)]
pub struct ResolutionStatistics {
    /// Total calls attempted
    pub total_attempts: usize,

    /// Successfully resolved calls
    pub resolved: usize,

    /// Failed resolutions
    pub failed: usize,

    /// Breakdown by strategy
    pub by_strategy: HashMap<ResolutionStrategy, StrategyStats>,

    /// Resolution time distribution
    pub time_percentiles: Percentiles,
}

#[derive(Debug, Clone, Default)]
pub struct StrategyStats {
    /// Times this strategy was tried
    pub attempts: usize,

    /// Times this strategy succeeded
    pub successes: usize,

    /// Times this strategy failed
    pub failures: usize,

    /// Average confidence when successful
    pub avg_confidence: f32,
}

/// Configuration for debug output
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Include successful resolutions (not just failures)
    pub show_successes: bool,

    /// Include timing information
    pub show_timing: bool,

    /// Maximum candidates to show per attempt
    pub max_candidates_shown: usize,

    /// Output format (text or json)
    pub format: DebugFormat,

    /// Only show attempts for specific functions
    pub filter_functions: Option<HashSet<String>>,
}

#[derive(Debug, Clone, Copy)]
pub enum DebugFormat {
    Text,
    Json,
}
```

### APIs and Interfaces

```rust
impl CallGraphDebugger {
    /// Create a new debugger with configuration
    pub fn new(config: DebugConfig) -> Self;

    /// Record a resolution attempt
    pub fn record_attempt(&mut self, attempt: ResolutionAttempt);

    /// Check if a function should be traced
    pub fn should_trace(&self, function_name: &str) -> bool;

    /// Get resolution statistics
    pub fn statistics(&self) -> &ResolutionStatistics;

    /// Get all failed resolutions
    pub fn failed_resolutions(&self) -> Vec<&ResolutionAttempt>;

    /// Generate debug report
    pub fn generate_report(&self) -> DebugReport;

    /// Output report to writer
    pub fn write_report<W: Write>(&self, writer: &mut W) -> Result<()>;
}

impl CallGraphValidator {
    /// Validate call graph structure
    pub fn validate(call_graph: &CallGraph) -> ValidationReport;

    /// Check for common issues
    pub fn check_heuristics(call_graph: &CallGraph) -> Vec<ValidationIssue>;

    /// Validate against expected patterns
    pub fn validate_expectations(
        call_graph: &CallGraph,
        expectations: &[Expectation],
    ) -> ValidationReport;
}

/// Validation report
#[derive(Debug)]
pub struct ValidationReport {
    /// Structural issues (dangling edges, etc.)
    pub structural_issues: Vec<StructuralIssue>,

    /// Heuristic warnings (suspicious patterns)
    pub warnings: Vec<ValidationWarning>,

    /// Overall health score (0-100)
    pub health_score: u32,
}

#[derive(Debug)]
pub enum StructuralIssue {
    /// Edge references non-existent node
    DanglingEdge {
        caller: FunctionId,
        callee: FunctionId,
    },

    /// Node exists but has no edges
    OrphanedNode {
        function: FunctionId,
    },

    /// Duplicate nodes
    DuplicateNode {
        function: FunctionId,
        count: usize,
    },
}

#[derive(Debug)]
pub enum ValidationWarning {
    /// Function has unexpectedly many callers
    TooManyCallers {
        function: FunctionId,
        count: usize,
    },

    /// Function has unexpectedly many callees
    TooManyCallees {
        function: FunctionId,
        count: usize,
    },

    /// All functions in file have 0 callers (suspicious)
    FileWithNoCalls {
        file: PathBuf,
        function_count: usize,
    },

    /// Public function has 0 callers
    UnusedPublicFunction {
        function: FunctionId,
    },
}
```

### Debug Output Format

**Text Format:**
```
🔍 Call Graph Debug Report
════════════════════════════════════════

📊 RESOLUTION STATISTICS
  Total Attempts:    1,234
  Resolved:          1,156 (93.7%)
  Failed:               78 (6.3%)

  By Strategy:
    ✓ Exact:           834 (72.2% of resolved, 98.5% success)
    ✓ Fuzzy:           256 (22.1% of resolved, 89.2% success)
    ✓ Name-Only:        66 (5.7% of resolved, 45.8% success)

  Resolution Time:
    p50: 0.12ms
    p95: 2.34ms
    p99: 8.91ms

❌ FAILED RESOLUTIONS (78 total)

  1. write_quick_wins_section
     Called from: EnhancedMarkdownWriter::write_executive_summary
     Location: src/io/writers/enhanced_markdown/mod.rs:126

     Strategy Attempts:
       1. Exact Match → No candidates
       2. Fuzzy Match → Found 2 candidates:
          • write_quick_wins_section (health_writer.rs:160)
          • write_quick_wins_section (legacy_writer.rs:98)
          Reason: AMBIGUOUS - Multiple matches, no clear winner
       3. Name-Only → Skipped (ambiguous)

     💡 Suggestion: Add module path or file hint for disambiguation

  2. format_output
     Called from: generate_report
     Location: src/output/mod.rs:45

     Strategy Attempts:
       1. Exact Match → No candidates
       2. Fuzzy Match → No candidates
       3. Name-Only → No candidates

     💡 Suggestion: Function may be in excluded crate or macro-generated

🔍 VALIDATION REPORT
  Health Score: 87/100

  Warnings (3):
    ⚠️  File with no incoming calls:
        src/deprecated/old_formatter.rs (5 functions)
        → May indicate dead code

    ⚠️  Public function with no callers:
        extract_metadata (src/analyzers/metadata.rs:23)
        → Verify if this is actually used

    ⚠️  Suspiciously many callees:
        process_analysis (src/core/analyzer.rs:145) - 87 callees
        → Consider refactoring this function

📈 RECOMMENDATIONS
  • 93.7% resolution rate is good (target: >95%)
  • Focus on improving fuzzy match disambiguation
  • Investigate 78 failed resolutions for patterns
  • Consider adding import context for ambiguous cases
```

**JSON Format:**
```json
{
  "statistics": {
    "total_attempts": 1234,
    "resolved": 1156,
    "failed": 78,
    "success_rate": 0.937,
    "by_strategy": {
      "Exact": { "attempts": 846, "successes": 834, "failures": 12 },
      "Fuzzy": { "attempts": 287, "successes": 256, "failures": 31 },
      "NameOnly": { "attempts": 144, "successes": 66, "failures": 78 }
    }
  },
  "failed_resolutions": [
    {
      "caller": {
        "function": "EnhancedMarkdownWriter::write_executive_summary",
        "file": "src/io/writers/enhanced_markdown/mod.rs",
        "line": 126
      },
      "callee_name": "write_quick_wins_section",
      "attempts": [
        {
          "strategy": "Exact",
          "candidates": [],
          "failure_reason": "NoCandidates"
        },
        {
          "strategy": "Fuzzy",
          "candidates": [
            {
              "name": "write_quick_wins_section",
              "file": "src/io/writers/enhanced_markdown/health_writer.rs",
              "line": 160
            },
            {
              "name": "write_quick_wins_section",
              "file": "src/io/writers/legacy/legacy_writer.rs",
              "line": 98
            }
          ],
          "failure_reason": {
            "Ambiguous": ["..."]
          }
        }
      ]
    }
  ],
  "validation": {
    "health_score": 87,
    "warnings": [
      {
        "type": "FileWithNoCalls",
        "file": "src/deprecated/old_formatter.rs",
        "function_count": 5
      }
    ]
  }
}
```

## Dependencies

- **Prerequisites**:
  - Spec 146 (Cross-Module Call Resolution) - Debug resolution process
  - Spec 148 (Enhanced FunctionId Matching) - Debug matching strategies
- **Affected Components**:
  - `src/analyzers/call_graph/call_resolution.rs` (add debug hooks)
  - `src/commands/analyze.rs` (add CLI flags)
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

1. **Debug Recording Tests** (`src/analyzers/call_graph/debug.rs`)
   ```rust
   #[test]
   fn test_record_resolution_attempt() {
       let mut debugger = CallGraphDebugger::new(DebugConfig::default());
       // Record attempts
       // Verify statistics are updated
   }

   #[test]
   fn test_statistics_calculation() {
       // Test success rate calculation
       // Test strategy breakdown
   }
   ```

2. **Validation Tests** (`src/analyzers/call_graph/validation.rs`)
   ```rust
   #[test]
   fn test_detect_dangling_edges() {
       // Create graph with dangling edge
       // Verify detected by validator
   }

   #[test]
   fn test_detect_orphaned_nodes() {
       // Create graph with orphaned nodes
       // Verify detected by validator
   }
   ```

### Integration Tests

1. **Debug Output Test** (`tests/call_graph_debug_output_test.rs`)
   - Run with `--debug-call-graph` flag
   - Verify output format matches specification
   - Check statistics are present and accurate

2. **Validation Test** (`tests/call_graph_validation_test.rs`)
   - Create test codebase with known issues
   - Run validation
   - Verify all issues are detected

### Performance Tests

1. **Debug Overhead Benchmark** (`benches/call_graph_bench.rs`)
   - Measure analysis time without debugging
   - Measure analysis time with debugging
   - Verify overhead < 20%

## Documentation Requirements

### Code Documentation

- Document CallGraphDebugger API
- Explain each validation check
- Provide examples of debug output interpretation

### User Documentation

**README.md sections to add:**

```markdown
### Debugging Call Graph Issues

If you notice functions incorrectly showing "0 callers", use debug mode:

\`\`\`bash
debtmap analyze . --debug-call-graph
\`\`\`

This will show:
- Which calls failed to resolve
- Why resolution failed
- Suggestions for fixing issues

To trace a specific function:

\`\`\`bash
debtmap analyze . --trace-function write_quick_wins_section
\`\`\`

For machine-readable output:

\`\`\`bash
debtmap analyze . --debug-call-graph --format json > debug.json
\`\`\`
```

### Architecture Updates

**ARCHITECTURE.md sections to add:**
- Debugging and Diagnostics → Call Graph Debug Tools
- Testing Strategy → Call Graph Validation

## Implementation Notes

### Recording Strategy

- Only record when debug mode enabled (no overhead otherwise)
- Use Arc<Mutex<Debugger>> for thread-safe recording
- Buffer attempts and flush periodically to avoid contention
- Limit memory usage (max N attempts stored)

### Performance Considerations

- Debug hooks should be no-ops when disabled
- Use conditional compilation for debug-only code
- Lazy evaluation of expensive debug info
- Configurable verbosity to control overhead

### Output Formatting

- Use consistent formatting with rest of debtmap
- Color-code by severity (errors, warnings, info)
- Provide actionable suggestions, not just problems
- Link to documentation for common issues

## Migration and Compatibility

### Breaking Changes

None - debug functionality is opt-in.

### Backward Compatibility

- Existing behavior unchanged when debug flags not used
- No changes to output format in normal mode

### Configuration

Add to Config struct:
```rust
pub struct Config {
    // ... existing fields ...

    /// Enable call graph debugging
    pub debug_call_graph: bool,

    /// Functions to trace
    pub trace_functions: Vec<String>,

    /// Debug output format
    pub debug_format: DebugFormat,
}
```

## Success Metrics

- **Primary**: Debug mode helps diagnose 100% of reported "0 callers" issues
- **Secondary**: Validation catches all known call graph bugs
- **Tertiary**: Performance overhead < 20% in debug mode
- **Developer**: Reduces debug time from hours to minutes

## Related Work

- Spec 146: Cross-Module Call Resolution (debugs resolution process)
- Spec 148: Enhanced FunctionId Matching (debugs matching strategies)
- Future: Interactive call graph visualization with drill-down
