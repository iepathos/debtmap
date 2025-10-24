---
number: 117
title: Enhanced Call Graph Display in Output
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-10-24
---

# Specification 117: Enhanced Call Graph Display in Output

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently collects comprehensive call graph data (callers and callees for each function) during analysis, and has formatting code to display this information in the `DEPENDENCIES` section of each recommendation. However, this valuable information is inconsistently displayed in the output, particularly in the top-level recommendations view.

**Current State**:
- Call graph data is extracted and stored in `upstream_callers` and `downstream_callees`
- A `format_dependencies_section` function exists that beautifully formats this data
- The dependencies section is conditionally included in output but often returns `None`
- Users cannot see the calling relationships that would help them understand:
  - Which functions depend on the flagged function (upstream callers)
  - Which functions the flagged function calls (downstream callees)
  - The impact radius of making changes to a function

**User's Example Request**:
From the user's debtmap output example, they want to see caller/callee information like:
```
#3 SCORE: 16.7 [🔴 UNTESTED] [CRITICAL]
├─ LOCATION: ./src/commands/analyze.rs:396 handle_call_graph_diagnostics()
├─ ACTION: Add 8 tests for 100% coverage gap, then refactor complexity 14 into 7 functions
├─ DEPENDENCIES:
│  ├─ 📞 Called by (2):
│  │     • main::cli_handler
│  │     • commands::run_analysis
│  └─ 📤 Calls (5):
│        • io::print_diagnostics
│        • analysis::validate_graph
│        • config::get_settings
│        ... (showing 3 of 5)
```

**Why This Matters**:
- **Impact assessment**: Understanding callers helps estimate refactoring blast radius
- **Priority decisions**: Functions with many callers may need different strategies
- **Dead code detection**: Functions with no callers are candidates for removal
- **Complexity understanding**: Functions calling many other functions may need decomposition
- **Test planning**: Knowing callers helps identify which tests to write
- **Refactoring safety**: Understanding dependencies prevents breaking changes

## Objective

Ensure that call graph data (callers and callees) is consistently displayed in all debtmap output formats, providing users with critical dependency information to make informed decisions about refactoring, testing, and prioritization.

## Requirements

### Functional Requirements

1. **Consistent Call Graph Display**
   - Show DEPENDENCIES section for all function-level debt items in terminal output
   - Display caller count and callee count even when zero
   - Include dependency information in JSON and Markdown output formats
   - Maintain existing filtering of standard library and external crate calls

2. **Enhanced Caller Information**
   - Show function name with module path (e.g., `module::submodule::function_name`)
   - Display file location for each caller (when available)
   - Indicate caller type (direct call, method call, trait implementation)
   - Sort callers by frequency of calls (most frequent first)
   - Show caller count even when list is empty ("No direct callers detected")

3. **Enhanced Callee Information**
   - Show function name with module path for each called function
   - Display file location for each callee (when available)
   - Filter out standard library and external crate calls by default
   - Indicate call type (function call, method call, macro invocation)
   - Show callee count even when list is empty or all filtered ("Calls no other functions")

4. **Display Configuration**
   - Control maximum number of callers/callees shown (default: 5)
   - Option to show/hide standard library calls
   - Option to show/hide external crate calls
   - Option to include file locations in caller/callee display
   - Verbosity control: compact view vs. detailed view

5. **Cross-File Call Graph**
   - Include calls across file boundaries
   - Show which files contain callers (multi-file analysis)
   - Indicate when calls cross module boundaries
   - Flag potential circular dependencies

6. **Output Format Consistency**
   - Terminal: Tree-structured visual display (existing format)
   - JSON: Structured arrays of caller/callee objects
   - Markdown: Formatted lists with links (for file-level items)
   - CSV: Comma-separated caller/callee lists

### Non-Functional Requirements

1. **Performance**
   - Call graph extraction should not add >10% to analysis time
   - Formatting dependencies should be O(n) where n is caller/callee count
   - Filtering should be lazy and efficient

2. **Accuracy**
   - 100% of direct calls should be detected
   - Method calls through traits should be marked as such
   - Dynamic dispatch should be flagged as uncertain

3. **Usability**
   - Clear visual distinction between callers and callees
   - Emoji/icons for better visual scanning
   - Truncation with clear indication of hidden items
   - Consistent formatting across all output types

4. **Maintainability**
   - Pure functions for formatting logic
   - Testable components for call graph extraction
   - Clear separation of concerns (extraction, filtering, formatting)

## Acceptance Criteria

- [ ] DEPENDENCIES section appears for 100% of function-level debt items in terminal output
- [ ] Caller count and callee count always displayed (even when 0)
- [ ] Callers displayed with format: `module::function_name` or `file.rs::function_name`
- [ ] Callees displayed with same format as callers
- [ ] Standard library calls filtered by default (configurable)
- [ ] External crate calls filtered by default (configurable)
- [ ] Maximum 5 callers/callees shown by default (configurable via config file)
- [ ] Truncation indicator when more than max items: `(showing 5 of 12)`
- [ ] "No direct callers detected" shown when caller list is empty
- [ ] "Calls no other functions" shown when callee list is empty (or all filtered)
- [ ] JSON output includes `callers` and `callees` arrays with full details
- [ ] Markdown output includes caller/callee sections with file links
- [ ] File-level debt items show aggregated caller/callee statistics
- [ ] Configuration option `[output.dependencies]` in `.debtmap.toml`
- [ ] CLI flag `--show-dependencies` / `--no-dependencies` overrides config
- [ ] CLI flag `--max-callers N` and `--max-callees N` for display limits
- [ ] Documentation updated with call graph display examples
- [ ] Tests verify dependency section appears in all expected outputs
- [ ] Tests verify filtering behavior (std lib, external crates)
- [ ] Tests verify truncation behavior
- [ ] Tests verify empty caller/callee handling

## Technical Details

### Implementation Approach

**Phase 1: Ensure Call Graph Data Population**
1. Verify call graph extraction is running for all analysis types
2. Ensure `upstream_callers` and `downstream_callees` are populated
3. Add defensive checks for missing call graph data
4. Add logging for call graph statistics

**Phase 2: Enhance Formatting Functions**
1. Update `format_dependencies_section` to always return `Some(_)`
2. Add formatting for empty caller/callee lists
3. Enhance caller/callee display with file locations
4. Improve visual formatting with better spacing

**Phase 3: Configuration System**
1. Add `[output.dependencies]` configuration section
2. Add CLI flags for dependency display control
3. Implement filtering configuration (std lib, external crates)
4. Add max display count configuration

**Phase 4: Multi-Format Support**
1. Enhance JSON output schema with caller/callee details
2. Update Markdown formatter with dependency sections
3. Ensure CSV output includes dependency information
4. Test all output formats for consistency

**Phase 5: Testing and Documentation**
1. Add unit tests for formatting functions
2. Add integration tests for end-to-end output
3. Update user documentation with examples
4. Add architecture documentation for call graph system

### Architecture Changes

```rust
// Enhanced configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyOutputConfig {
    /// Whether to show the DEPENDENCIES section
    pub enabled: bool,

    /// Maximum number of callers to display
    pub max_callers: usize,

    /// Maximum number of callees to display
    pub max_callees: usize,

    /// Whether to filter standard library calls
    pub filter_std_lib: bool,

    /// Whether to filter external crate calls
    pub filter_external_crates: bool,

    /// Whether to show file locations for callers/callees
    pub show_file_locations: bool,

    /// Whether to show full module paths
    pub show_full_paths: bool,
}

impl Default for DependencyOutputConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_callers: 5,
            max_callees: 5,
            filter_std_lib: true,
            filter_external_crates: true,
            show_file_locations: false,
            show_full_paths: false,
        }
    }
}

// Enhanced caller/callee information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSiteInfo {
    /// Function name (potentially with module path)
    pub function_name: String,

    /// File containing the caller/callee
    pub file: Option<PathBuf>,

    /// Line number where call occurs
    pub line: Option<usize>,

    /// Type of call (direct, method, trait, macro)
    pub call_type: CallType,

    /// Number of times this call occurs (if tracked)
    pub call_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CallType {
    Direct,           // Regular function call
    Method,           // Method call on instance
    TraitMethod,      // Method call through trait
    MacroInvocation,  // Called via macro
    DynamicDispatch,  // Dynamic dispatch (uncertain)
}

// Update DependencyInfo to use CallSiteInfo
#[derive(Debug, Clone, Default)]
pub struct DependencyInfo {
    /// Functions that call this function
    pub upstream_callers: Vec<CallSiteInfo>,

    /// Functions called by this function
    pub downstream_callees: Vec<CallSiteInfo>,

    /// Whether dependency info is complete
    pub has_dependencies: bool,
}
```

### Data Structures

Current structure in `UnifiedDebtItem`:
```rust
pub struct UnifiedDebtItem {
    // ... existing fields ...
    pub upstream_callers: Vec<String>,
    pub downstream_callees: Vec<String>,
}
```

Enhanced structure (backward compatible):
```rust
pub struct UnifiedDebtItem {
    // ... existing fields ...

    /// Simple string list (deprecated, for backward compatibility)
    pub upstream_callers: Vec<String>,
    pub downstream_callees: Vec<String>,

    /// Enhanced caller information with file locations
    pub caller_details: Option<Vec<CallSiteInfo>>,
    pub callee_details: Option<Vec<CallSiteInfo>>,
}
```

### APIs and Interfaces

**Configuration in .debtmap.toml**:
```toml
[output.dependencies]
enabled = true
max_callers = 5
max_callees = 5
filter_std_lib = true
filter_external_crates = true
show_file_locations = false
show_full_paths = false
```

**CLI Options**:
```bash
# Show dependencies (default: true)
debtmap analyze src --show-dependencies

# Hide dependencies
debtmap analyze src --no-dependencies

# Control display limits
debtmap analyze src --max-callers 10 --max-callees 10

# Show file locations for callers/callees
debtmap analyze src --show-caller-locations

# Include standard library calls
debtmap analyze src --include-std-lib-calls
```

**JSON Output Format**:
```json
{
  "rank": 3,
  "score": 16.7,
  "function": "handle_call_graph_diagnostics",
  "file": "./src/commands/analyze.rs",
  "line": 396,
  "dependencies": {
    "callers": [
      {
        "function_name": "cli_handler",
        "file": "./src/main.rs",
        "line": 45,
        "call_type": "Direct",
        "call_count": 1
      },
      {
        "function_name": "run_analysis",
        "file": "./src/commands/mod.rs",
        "line": 123,
        "call_type": "Direct",
        "call_count": 1
      }
    ],
    "callees": [
      {
        "function_name": "print_diagnostics",
        "file": "./src/io/mod.rs",
        "line": 89,
        "call_type": "Direct",
        "call_count": 3
      },
      {
        "function_name": "validate_graph",
        "file": "./src/analysis/mod.rs",
        "line": 234,
        "call_type": "Direct",
        "call_count": 1
      }
    ],
    "caller_count": 2,
    "callee_count": 5,
    "truncated": true
  }
}
```

**Terminal Output Format**:
```
#3 SCORE: 16.7 [🔴 UNTESTED] [CRITICAL]
   ↳ Main factors: 🔴 UNTESTED (0% coverage, weight: 50%), Moderate complexity
├─ LOCATION: ./src/commands/analyze.rs:396 handle_call_graph_diagnostics()
├─ ACTION: Add 8 tests for 100% coverage gap, then refactor complexity 14 into 7 functions
├─ DEPENDENCIES:
│  ├─ 📞 Called by (2):
│  │     • main::cli_handler
│  │     • commands::run_analysis
│  └─ 📤 Calls (5):
│        • io::print_diagnostics
│        • analysis::validate_graph
│        • config::get_settings
│        … (showing 3 of 5)
```

Or with file locations enabled:
```
├─ DEPENDENCIES:
│  ├─ 📞 Called by (2):
│  │     • main::cli_handler (src/main.rs:45)
│  │     • commands::run_analysis (src/commands/mod.rs:123)
│  └─ 📤 Calls (5):
│        • io::print_diagnostics (src/io/mod.rs:89)
│        • analysis::validate_graph (src/analysis/mod.rs:234)
│        • config::get_settings (src/config.rs:56)
│        … (showing 3 of 5)
```

For empty callers:
```
├─ DEPENDENCIES:
│  ├─ 📞 Called by: No direct callers detected
│  └─ 📤 Calls (3):
│        • helper::process_data
│        • utils::validate_input
│        • io::write_output
```

For functions that call nothing:
```
├─ DEPENDENCIES:
│  ├─ 📞 Called by (2):
│  │     • main::process
│  │     • worker::run_task
│  └─ 📤 Calls: Calls no other functions
```

### Code Organization

```
src/priority/
├─ call_graph/
│  ├─ extraction.rs      # Call graph extraction logic
│  ├─ types.rs           # CallSiteInfo, CallType definitions
│  ├─ filtering.rs       # Filtering std lib, external crates
│  └─ tests.rs           # Call graph extraction tests
├─ formatter.rs          # Main formatting (already exists)
│  ├─ format_dependencies_section()  # Enhanced to always return Some
│  ├─ format_caller_info()           # New: format single caller
│  ├─ format_callee_info()           # New: format single callee
│  └─ filter_dependencies()          # Already exists
└─ formatter_tests.rs    # Enhanced tests for dependencies
```

## Dependencies

- **Prerequisites**: None (builds on existing call graph functionality)
- **Affected Components**:
  - `src/priority/formatter.rs` - Enhance dependency formatting
  - `src/priority/call_graph/` - May need enhancements for file location tracking
  - `src/config.rs` - Add dependency output configuration
  - `src/io/json_output.rs` - Add dependency info to JSON schema
  - `src/io/markdown_output.rs` - Add dependency sections
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_dependencies_always_returns_some() {
        let context = FormatContext {
            dependency_info: DependencyInfo {
                upstream_callers: vec![],
                downstream_callees: vec![],
                has_dependencies: true,
            },
            ..Default::default()
        };

        let result = format_dependencies_section(&context);
        assert!(result.is_some());
        assert!(result.unwrap().contains("No direct callers detected"));
    }

    #[test]
    fn test_format_dependencies_with_callers_and_callees() {
        let context = FormatContext {
            dependency_info: DependencyInfo {
                upstream_callers: vec![
                    CallSiteInfo {
                        function_name: "main::process".to_string(),
                        file: Some(PathBuf::from("src/main.rs")),
                        line: Some(45),
                        call_type: CallType::Direct,
                        call_count: Some(1),
                    },
                ],
                downstream_callees: vec![
                    CallSiteInfo {
                        function_name: "utils::validate".to_string(),
                        file: Some(PathBuf::from("src/utils.rs")),
                        line: Some(123),
                        call_type: CallType::Direct,
                        call_count: Some(1),
                    },
                ],
                has_dependencies: true,
            },
            ..Default::default()
        };

        let result = format_dependencies_section(&context).unwrap();
        assert!(result.contains("DEPENDENCIES:"));
        assert!(result.contains("Called by (1)"));
        assert!(result.contains("main::process"));
        assert!(result.contains("Calls (1)"));
        assert!(result.contains("utils::validate"));
    }

    #[test]
    fn test_filter_std_lib_calls() {
        let callees = vec![
            CallSiteInfo {
                function_name: "std::println".to_string(),
                ..Default::default()
            },
            CallSiteInfo {
                function_name: "my_module::my_function".to_string(),
                ..Default::default()
            },
        ];

        let config = DependencyOutputConfig {
            filter_std_lib: true,
            ..Default::default()
        };

        let filtered = filter_dependency_info(&callees, &config);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].function_name, "my_module::my_function");
    }

    #[test]
    fn test_truncation_display() {
        let callers = (0..10)
            .map(|i| CallSiteInfo {
                function_name: format!("caller_{}", i),
                ..Default::default()
            })
            .collect();

        let context = FormatContext {
            dependency_info: DependencyInfo {
                upstream_callers: callers,
                downstream_callees: vec![],
                has_dependencies: true,
            },
            ..Default::default()
        };

        let config = DependencyOutputConfig {
            max_callers: 5,
            ..Default::default()
        };

        let result = format_dependencies_section_with_config(&context, config).unwrap();
        assert!(result.contains("(showing 5 of 10)"));
    }

    #[test]
    fn test_file_location_display() {
        let caller = CallSiteInfo {
            function_name: "main::process".to_string(),
            file: Some(PathBuf::from("src/main.rs")),
            line: Some(45),
            call_type: CallType::Direct,
            call_count: Some(1),
        };

        let context = FormatContext {
            dependency_info: DependencyInfo {
                upstream_callers: vec![caller],
                downstream_callees: vec![],
                has_dependencies: true,
            },
            ..Default::default()
        };

        let config = DependencyOutputConfig {
            show_file_locations: true,
            ..Default::default()
        };

        let result = format_dependencies_section_with_config(&context, config).unwrap();
        assert!(result.contains("main::process"));
        assert!(result.contains("src/main.rs:45"));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_analyze_shows_dependencies_for_all_items() {
    let output = run_debtmap_analyze("tests/fixtures/sample_project");
    let items = parse_output_items(&output);

    for item in items {
        // Every function-level item should have a DEPENDENCIES section
        if item.is_function_level() {
            assert!(
                item.text.contains("DEPENDENCIES:"),
                "Item {} missing DEPENDENCIES section",
                item.rank
            );
        }
    }
}

#[test]
fn test_json_output_includes_dependencies() {
    let output = run_debtmap_analyze_json("tests/fixtures/sample_project");
    let json: serde_json::Value = serde_json::from_str(&output).unwrap();

    let items = json["items"].as_array().unwrap();
    for item in items {
        if item["type"] == "function" {
            assert!(item["dependencies"].is_object());
            assert!(item["dependencies"]["callers"].is_array());
            assert!(item["dependencies"]["callees"].is_array());
        }
    }
}
```

## Documentation Requirements

### User Documentation

```markdown
## Call Graph Display

Debtmap shows caller and callee relationships for each function in the DEPENDENCIES section.

### Reading the Dependencies Section

```
├─ DEPENDENCIES:
│  ├─ 📞 Called by (2):     # Functions that call this function
│  │     • main::cli_handler
│  │     • commands::run_analysis
│  └─ 📤 Calls (5):         # Functions called by this function
│        • io::print_diagnostics
│        • analysis::validate_graph
│        … (showing 3 of 5)  # Truncated display
```

### Configuration

Control dependency display in `.debtmap.toml`:

```toml
[output.dependencies]
enabled = true                  # Show dependencies section
max_callers = 5                # Maximum callers to display
max_callees = 5                # Maximum callees to display
filter_std_lib = true          # Hide standard library calls
filter_external_crates = true  # Hide external crate calls
show_file_locations = false    # Show file:line for each call
show_full_paths = false        # Show full module paths
```

### CLI Options

```bash
# Hide dependencies
debtmap analyze src --no-dependencies

# Show more callers/callees
debtmap analyze src --max-callers 10 --max-callees 10

# Show file locations
debtmap analyze src --show-caller-locations

# Include standard library calls
debtmap analyze src --include-std-lib-calls
```

### Understanding Dependency Information

- **No direct callers detected**: Function may be dead code or entry point
- **Calls no other functions**: Leaf function, minimal dependencies
- **Many callers (>10)**: High-impact function, changes affect many places
- **Many callees (>10)**: Complex function, candidate for decomposition
```

## Implementation Notes

### Backward Compatibility

- Maintain existing `upstream_callers` and `downstream_callees` as `Vec<String>`
- Add optional `caller_details` and `callee_details` for enhanced information
- Default to showing dependencies (existing behavior where available)
- Configuration allows disabling if users prefer compact output

### Performance Considerations

- Call graph extraction already happens, no additional analysis needed
- Formatting is O(n) where n is number of callers/callees
- Filtering should be lazy (only when displaying)
- Truncation prevents overwhelming output for highly-connected functions

### Edge Cases

1. **Recursive functions**: Show self-call in callees
2. **Indirect recursion**: Mark with special indicator
3. **Dynamic dispatch**: Mark with `[dynamic]` or uncertainty indicator
4. **Macro-generated calls**: Mark with `[macro]` indicator
5. **Cross-crate calls**: Include if not filtered
6. **Incomplete call graph**: Show warning if graph is partial

## Migration and Compatibility

### Breaking Changes
None. This is purely additive functionality.

### Deprecations
None. Enhances existing functionality.

### Migration Path
No migration needed. Users automatically get enhanced output on upgrade.

### Rollback Plan
Configuration option `dependencies.enabled = false` disables the feature.

## Success Metrics

- **Adoption**: 80% of users keep dependencies section enabled
- **Usefulness**: User feedback indicates dependency info is helpful
- **Performance**: <5% increase in total analysis time
- **Accuracy**: 100% of direct calls shown (for analyzed code)
- **Clarity**: Users can quickly identify high-impact functions by caller count

## Future Enhancements

Potential future additions (out of scope for this spec):

1. **Interactive call graph**: Click to navigate caller/callee chain
2. **Call frequency**: Show how often each caller invokes the function
3. **Call context**: Show code snippet around each call site
4. **Circular dependency detection**: Highlight circular call chains
5. **Call graph visualization**: Generate graphviz/mermaid diagrams
6. **Cross-language calls**: Track calls between Rust/Python/JS/TS
7. **Test coverage impact**: Show which tests exercise which call paths
