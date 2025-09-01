---
number: 20
title: Priority Index Flag for Parallel Processing
category: feature
priority: medium
status: draft
dependencies: [19]
created: 2025-08-12
---

# Specification 20: Priority Index Flag for Parallel Processing

**Category**: feature
**Priority**: medium
**Status**: draft
**Dependencies**: [19 - Unified Debt Prioritization with Semantic Analysis]

## Context

When using debtmap with prodigy's `--map-args` feature for parallel processing, users need a way to extract individual priority items by their index position. Currently, the `--top N` flag returns multiple items, but there's no way to get just the Nth priority item for distributed processing.

This limitation prevents efficient parallelization where different processes could work on different priority items simultaneously, such as:
- Parallel testing of high-priority functions
- Distributed refactoring efforts across a team
- CI/CD pipeline stages that focus on specific priority levels

## Objective

Add a `--priority-index` CLI flag that returns only the Nth priority item (1-based indexing) from the unified debt analysis, enabling efficient parallel processing when combined with prodigy's `--map-args` feature.

## Requirements

### Functional Requirements

1. **Priority Index Selection**
   - Accept 1-based index values (1 = highest priority item)
   - Return exactly one priority item when index exists
   - Return appropriate error/empty result when index is out of bounds
   - Maintain consistent ordering with existing `--top N` output

2. **Integration with Existing Flags**
   - Work seamlessly with `--format` flag (json, markdown, terminal)
   - Compatible with `--top` flag (but mutually exclusive - `--priority-index` takes precedence)
   - Honor `--priorities-only` and `--detailed` formatting options
   - Respect existing filtering and threshold settings

3. **Error Handling**
   - Clear error message when index is out of bounds
   - Graceful handling of empty analysis results
   - Informative messaging when no priority items exist
   - Non-zero exit code for invalid index values

4. **Output Consistency**
   - Single item formatted identically to multi-item output
   - Include context about total available items when appropriate
   - Maintain all metadata (location, score, recommendation, etc.)

### Non-Functional Requirements

1. **Performance**
   - Efficient extraction without processing all items if possible
   - No significant performance impact on analysis pipeline
   - Minimal memory overhead for index-based access

2. **Usability**
   - Clear help text with examples
   - Intuitive 1-based indexing matching human expectations
   - Informative error messages guiding correct usage

3. **Compatibility**
   - No breaking changes to existing CLI interface
   - Maintains compatibility with existing output formats
   - Works with all current configuration options

## Acceptance Criteria

- [ ] `--priority-index 1` returns the highest priority item
- [ ] `--priority-index N` returns the Nth priority item (1-based)
- [ ] Out-of-bounds indices return appropriate error messages and non-zero exit codes
- [ ] Works with all output formats: `--format json`, `--format markdown`, `--format terminal`
- [ ] Compatible with `--priorities-only` and `--detailed` formatting modes
- [ ] Single item output maintains same structure as multi-item output
- [ ] Error messages include helpful context about available range
- [ ] Help text clearly documents usage and examples
- [ ] Integration tests demonstrate parallel processing scenarios with prodigy
- [ ] Performance impact is negligible compared to full analysis

## Technical Details

### Implementation Approach

#### 1. CLI Interface Enhancement (`src/cli.rs`)

Add the new flag to both Analyze and Validate commands:

```rust
/// Extract specific priority item by index (1-based) for parallel processing
#[arg(long = "priority-index", conflicts_with = "top")]
priority_index: Option<usize>,
```

**Key Design Decisions:**
- 1-based indexing for user-friendliness (matches common human counting)
- Conflicts with `--top` to avoid ambiguous behavior
- Optional field that defaults to None (preserves existing behavior)

#### 2. Priority Extraction Logic (`src/priority/mod.rs`)

Enhance UnifiedAnalysis with index-based access:

```rust
impl UnifiedAnalysis {
    /// Get priority item by 1-based index
    pub fn get_priority_by_index(&self, index: usize) -> Option<UnifiedDebtItem> {
        if index == 0 {
            return None; // Invalid 1-based index
        }
        self.items.get(index - 1).cloned()
    }
    
    /// Get total count of priority items
    pub fn priority_count(&self) -> usize {
        self.items.len()
    }
}
```

#### 3. Output Formatting Updates (`src/priority/formatter.rs`)

Add new output format variant:

```rust
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Default,        // Top 10 with clean formatting
    PrioritiesOnly, // Minimal list
    Detailed,       // Full analysis with priority overlay
    Top(usize),     // Top N items
    Index(usize),   // Single item by index (new)
}

pub fn format_priorities(analysis: &UnifiedAnalysis, format: OutputFormat) -> String {
    match format {
        OutputFormat::Default => format_default(analysis, 10),
        OutputFormat::PrioritiesOnly => format_priorities_only(analysis, 10),
        OutputFormat::Detailed => format_detailed(analysis),
        OutputFormat::Top(n) => format_default(analysis, n),
        OutputFormat::Index(index) => format_single_priority(analysis, index),
    }
}

fn format_single_priority(analysis: &UnifiedAnalysis, index: usize) -> String {
    let mut output = String::new();
    
    match analysis.get_priority_by_index(index) {
        Some(item) => {
            writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
            writeln!(output, "    {}", format!("PRIORITY ITEM #{index}").bright_white().bold()).unwrap();
            writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
            writeln!(output).unwrap();
            
            format_priority_item(&mut output, index, &item);
            
            // Add context about total items
            writeln!(output).unwrap();
            writeln!(
                output, 
                "📊 {} (showing item {index} of {})",
                "CONTEXT".bright_cyan().bold(),
                analysis.priority_count()
            ).unwrap();
        }
        None => {
            let total = analysis.priority_count();
            writeln!(output, "{}", "⚠️  INDEX OUT OF BOUNDS".bright_yellow().bold()).unwrap();
            writeln!(output).unwrap();
            
            if total == 0 {
                writeln!(output, "No priority items found in analysis.").unwrap();
                writeln!(output, "Try adjusting analysis parameters or check if files contain debt.").unwrap();
            } else {
                writeln!(output, "Requested index: {index}").unwrap();
                writeln!(output, "Available range: 1-{total}").unwrap();
                writeln!(output).unwrap();
                writeln!(output, "Examples:").unwrap();
                writeln!(output, "  debtmap analyze . --priority-index 1    # Highest priority").unwrap();
                writeln!(output, "  debtmap analyze . --priority-index {total}    # Lowest priority").unwrap();
            }
        }
    }
    
    output
}
```

#### 4. JSON and Markdown Format Support

Ensure single-item output works correctly with structured formats:

```rust
// JSON format for single item
{
    "priority_item": {
        "index": 1,
        "total_count": 25,
        "item": {
            // ... existing UnifiedDebtItem structure
        }
    }
}

// Markdown format for single item  
# Priority Item #1

**Score:** 9.2 [CRITICAL]

**Location:** `src/payment/processor.rs:45` - `process_payment()`

**Action:** Add comprehensive unit tests

...

*Context: Showing item 1 of 25 total priority items*
```

#### 5. Error Handling and Validation

```rust
pub fn validate_priority_index(
    index: Option<usize>, 
    analysis: &UnifiedAnalysis
) -> Result<(), String> {
    match index {
        None => Ok(()),
        Some(0) => Err("Priority index must be >= 1 (1-based indexing)".to_string()),
        Some(i) if i > analysis.priority_count() => {
            Err(format!(
                "Index {} out of bounds. Available range: 1-{}",
                i, analysis.priority_count()
            ))
        }
        Some(_) => Ok(()),
    }
}
```

### Architecture Changes

#### Modified Components
- `src/cli.rs`: Add `--priority-index` flag to Analyze and Validate commands
- `src/priority/mod.rs`: Add index-based access methods to UnifiedAnalysis
- `src/priority/formatter.rs`: Add single-item formatting support
- `src/main.rs`: Integrate priority index logic into analysis pipeline

#### New Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum PriorityIndexError {
    #[error("Priority index must be >= 1 (1-based indexing)")]
    InvalidIndex,
    
    #[error("Index {index} out of bounds. Available range: 1-{max}")]
    OutOfBounds { index: usize, max: usize },
    
    #[error("No priority items found in analysis")]
    EmptyAnalysis,
}
```

### Integration Points

#### Command Line Processing
```rust
// In main analysis flow
if let Some(index) = args.priority_index {
    validate_priority_index(Some(index), &analysis)?;
    let format = OutputFormat::Index(index);
    let output = format_priorities(&analysis, format);
    println!("{}", output);
    
    // Exit with error code if index was out of bounds
    if analysis.get_priority_by_index(index).is_none() {
        std::process::exit(1);
    }
} else if let Some(top) = args.top {
    // Existing top N logic
} else {
    // Default behavior
}
```

## Dependencies

### Prerequisites
- **Spec 19**: Unified Debt Prioritization with Semantic Analysis
  - Provides the UnifiedAnalysis and priority scoring system
  - Required for consistent priority item access

### Affected Components
- `src/cli.rs`: CLI flag definitions
- `src/priority/mod.rs`: Core analysis types
- `src/priority/formatter.rs`: Output formatting
- `src/main.rs`: Main analysis pipeline
- Integration tests in `tests/priority_integration.rs`

### External Dependencies
- No new external crates required
- Uses existing clap, colored, and serde dependencies

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_priority_by_index_valid() {
        let mut analysis = create_test_analysis_with_items(3);
        
        let item = analysis.get_priority_by_index(1);
        assert!(item.is_some());
        
        let item = analysis.get_priority_by_index(3);
        assert!(item.is_some());
    }

    #[test]
    fn test_get_priority_by_index_out_of_bounds() {
        let mut analysis = create_test_analysis_with_items(2);
        
        // Test 0-based (invalid)
        assert!(analysis.get_priority_by_index(0).is_none());
        
        // Test beyond range
        assert!(analysis.get_priority_by_index(3).is_none());
        
        // Test empty analysis
        let empty_analysis = UnifiedAnalysis::new(CallGraph::new());
        assert!(empty_analysis.get_priority_by_index(1).is_none());
    }

    #[test]
    fn test_format_single_priority_valid_index() {
        let analysis = create_test_analysis_with_items(3);
        let output = format_single_priority(&analysis, 1);
        
        assert!(output.contains("PRIORITY ITEM #1"));
        assert!(output.contains("showing item 1 of 3"));
    }

    #[test]
    fn test_format_single_priority_out_of_bounds() {
        let analysis = create_test_analysis_with_items(2);
        let output = format_single_priority(&analysis, 5);
        
        assert!(output.contains("INDEX OUT OF BOUNDS"));
        assert!(output.contains("Available range: 1-2"));
        assert!(output.contains("Requested index: 5"));
    }

    #[test]
    fn test_priority_index_cli_parsing() {
        use clap::Parser;
        
        let args = vec!["debtmap", "analyze", ".", "--priority-index", "5"];
        let cli = Cli::parse_from(args);
        
        match cli.command {
            Commands::Analyze { priority_index, .. } => {
                assert_eq!(priority_index, Some(5));
            }
            _ => panic!("Expected Analyze command"),
        }
    }

    #[test]
    fn test_priority_index_conflicts_with_top() {
        use clap::Parser;
        
        // This should fail due to conflicts_with annotation
        let args = vec!["debtmap", "analyze", ".", "--priority-index", "1", "--top", "5"];
        let result = Cli::try_parse_from(args);
        
        assert!(result.is_err());
    }
}
```

### Integration Tests

```rust
// tests/priority_index_integration.rs
#[test]
fn test_priority_index_with_real_codebase() {
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/sample_rust", "--priority-index", "1"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("PRIORITY ITEM #1"));
    assert!(stdout.contains("CONTEXT"));
}

#[test]
fn test_priority_index_out_of_bounds_error() {
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/empty_project", "--priority-index", "1"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(!output.status.success());
    assert_eq!(output.status.code().unwrap(), 1);
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("No priority items found"));
}

#[test]
fn test_priority_index_with_json_format() {
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/sample_rust", "--priority-index", "2", "--format", "json"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    
    assert!(json["priority_item"]["index"].as_u64().unwrap() == 2);
    assert!(json["priority_item"]["item"]["unified_score"].is_object());
}
```

### Parallel Processing Integration Tests

```rust
#[test]
fn test_prodigy_integration_example() {
    // Demonstrate how this would work with prodigy --map-args
    
    // First, get the count of priority items
    let count_output = Command::new("./target/debug/debtmap")
        .args(&["analyze", ".", "--top", "1000", "--format", "json"])
        .output()
        .expect("Failed to get count");
    
    let json: serde_json::Value = serde_json::from_str(
        &String::from_utf8(count_output.stdout).unwrap()
    ).unwrap();
    
    let item_count = json["priority_analysis"]["items"].as_array().unwrap().len();
    
    // Now test that each index works
    for i in 1..=std::cmp::min(item_count, 5) {
        let output = Command::new("./target/debug/debtmap")
            .args(&["analyze", ".", "--priority-index", &i.to_string()])
            .output()
            .expect("Failed to execute debtmap");

        assert!(output.status.success(), "Failed for index {}", i);
        
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains(&format!("PRIORITY ITEM #{}", i)));
    }
}
```

## Documentation Requirements

### Help Text
```
--priority-index <INDEX>
    Extract specific priority item by index (1-based) for parallel processing.
    
    Use with prodigy's --map-args for distributed analysis:
    
    Examples:
      --priority-index 1                    Get highest priority item
      --priority-index 5 --format json     Get 5th item as JSON
      --priority-index 10 --priorities-only Get 10th item (minimal format)
    
    prodigy Integration:
      prodigy "debtmap analyze . --priority-index {}" --map-args 1,2,3,4,5
    
    Notes:
      - Uses 1-based indexing (1 = highest priority)
      - Returns error if index is out of bounds
      - Conflicts with --top flag
      - Works with all output formats and detail levels
```

### User Documentation Updates

Add to README.md:
```markdown
## Parallel Processing with prodigy

For large codebases or distributed teams, you can process priority items in parallel:

```bash
# Get individual priority items
debtmap analyze . --priority-index 1     # Highest priority
debtmap analyze . --priority-index 2     # Second priority
# ... etc

# Use with prodigy for parallel processing
prodigy "debtmap analyze . --priority-index {} --format json" --map-args $(seq 1 10)
```

This enables scenarios like:
- Parallel testing of high-priority functions
- Distributed refactoring across team members  
- CI/CD stages focusing on different priority levels
```

### Architecture Documentation Updates

Update ARCHITECTURE.md to document the new priority access patterns and their integration with the existing priority system.

## Implementation Notes

### Phased Implementation
1. **Phase 1**: Basic CLI flag and index access method
2. **Phase 2**: Single-item formatting for terminal output
3. **Phase 3**: JSON and Markdown format support
4. **Phase 4**: Enhanced error handling and validation
5. **Phase 5**: Integration tests and documentation

### Edge Cases to Consider
- Empty analysis (no priority items found)
- Single priority item (index 1 is only valid option)
- Very large indices (efficient bounds checking)
- Concurrent access patterns with multiple processes

### Performance Optimizations
- Lazy evaluation: don't format items that won't be shown
- Efficient indexing: use Vector's O(1) access rather than iteration
- Memory efficiency: clone only the requested item

## Usage Examples

### Basic Usage
```bash
# Get the highest priority item
debtmap analyze . --priority-index 1

# Get the 5th priority item in JSON format
debtmap analyze . --priority-index 5 --format json

# Get 3rd priority item with detailed breakdown
debtmap analyze . --priority-index 3 --detailed
```

### Parallel Processing with prodigy
```bash
# Process top 5 priority items in parallel
prodigy "debtmap analyze . --priority-index {} --format json" --map-args 1,2,3,4,5

# Parallel refactoring: each team member gets different priorities
prodigy "echo 'Priority {}: $(debtmap analyze . --priority-index {} --priorities-only)'" --map-args 1,2,3

# CI/CD: Different build stages handle different priority levels
prodigy "debtmap analyze . --priority-index {} | ./handle-priority.sh {}" --map-args $(seq 1 10)
```

### Error Handling
```bash
# Out of bounds - exits with code 1
debtmap analyze empty-project --priority-index 1
# Output: "No priority items found in analysis."

# Invalid index - exits with code 1  
debtmap analyze . --priority-index 999
# Output: "Index 999 out of bounds. Available range: 1-25"
```

## Expected Impact

After implementation:

1. **Enhanced Parallel Processing**: Teams can efficiently distribute priority item analysis across multiple processes or team members
2. **Better prodigy Integration**: Seamless integration with prodigy's `--map-args` feature for distributed processing
3. **Improved Workflow Automation**: CI/CD pipelines can target specific priority levels without processing entire analysis
4. **Maintained Consistency**: Single-item output maintains the same structure and metadata as multi-item output
5. **Clear Error Guidance**: Users receive helpful feedback when indices are out of bounds or invalid

This feature enables efficient parallel processing workflows while maintaining debtmap's existing functionality and user experience patterns.

## Migration and Compatibility

- **Breaking Changes**: None - purely additive feature
- **Configuration Migration**: No configuration changes required
- **Output Compatibility**: New single-item output format maintains consistency with existing formats
- **API Stability**: No changes to existing CLI options or behavior

The `--priority-index` flag provides a focused way to extract individual priority items, enabling new parallel processing workflows while preserving all existing functionality.