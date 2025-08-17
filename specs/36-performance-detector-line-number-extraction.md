---
number: 36
title: Performance Detector Line Number Extraction
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-08-17
---

# Specification 36: Performance Detector Line Number Extraction

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: []

## Context

Currently, the performance pattern detection system has a critical bug that causes false positives by assigning arbitrary line numbers (idx + 1) to detected patterns instead of extracting actual source code locations where performance issues occur. This results in performance issues being incorrectly attributed to import statements, function signatures, and other non-problematic code locations.

### Current Problem

In `src/analyzers/rust.rs:analyze_performance_patterns()`, the code uses placeholder line numbers:

```rust
for (idx, pattern) in anti_patterns.into_iter().enumerate() {
    let impact = detector.estimate_impact(&pattern);
    // Use a placeholder line number since we don't have exact positions yet
    let line = idx + 1;  // ← BUG: Arbitrary line assignment
    let debt_item = convert_performance_pattern_to_debt_item(pattern, impact, path, line);
    performance_items.push(debt_item);
}
```

This causes:
1. **False Positives**: Import statements flagged as "Blocking I/O" performance issues
2. **Misleading Reports**: Performance issues attributed to wrong source locations
3. **Developer Confusion**: Debugging effort wasted on non-problematic code
4. **Reduced Trust**: Tool credibility damaged by obvious false positives

### Impact Assessment

- Critical severity affecting all performance analysis
- Performance detectors (IOPerformanceDetector, StringPerformanceDetector, etc.) find legitimate patterns but assign wrong line numbers
- Issues manifest as imports being flagged with "High" priority performance problems
- Temporary fix implemented: performance patterns are skipped until proper line extraction is available

## Objective

Implement comprehensive line number extraction for all performance pattern detectors to:

1. **Accurate Location Reporting**: Extract actual source line numbers where performance patterns are detected
2. **Eliminate False Positives**: Ensure performance issues are only reported at their true source locations
3. **Enhanced Debugging**: Provide precise source locations for developers to investigate performance issues
4. **Restore Performance Analysis**: Re-enable all performance detectors with accurate line information

## Requirements

### Functional Requirements

1. **AST Line Number Extraction**
   - Extract actual line numbers from syn::Span information for all detected patterns
   - Support for multi-line patterns (report starting line, optionally include range)
   - Handle macro-expanded code with proper source location mapping
   - Graceful fallback when line information is unavailable

2. **Pattern-Specific Location Tracking**
   - **Nested Loops**: Report line of innermost loop causing performance concern
   - **I/O Operations**: Report line of blocking I/O call (File::open, read, write, etc.)
   - **String Operations**: Report line of inefficient string manipulation
   - **Data Structure Operations**: Report line of inefficient collection usage
   - **Allocation Patterns**: Report line of problematic memory allocation

3. **Enhanced Pattern Data Structure**
   - Include source location information in PerformanceAntiPattern variants
   - Maintain backward compatibility with existing pattern detection logic
   - Support optional line ranges for multi-line patterns
   - Include column information when available for precise positioning

4. **Accurate Debt Item Creation**
   - Use extracted line numbers instead of arbitrary indices
   - Preserve all existing metadata (pattern type, impact, severity)
   - Include source context when available (function name, surrounding code)
   - Support confidence scoring based on location accuracy

### Non-Functional Requirements

1. **Performance**
   - Line extraction adds <5% overhead to analysis time
   - Efficient span information retrieval from syn AST
   - Minimal memory overhead for storing location information
   - No impact on non-performance analysis workflows

2. **Accuracy**
   - 100% accuracy for extractable line numbers from syn spans
   - Graceful handling of unavailable location information
   - No false line number assignments
   - Clear indication when line information is uncertain

3. **Maintainability**
   - Clean separation between pattern detection and location extraction
   - Consistent location extraction patterns across all detectors
   - Comprehensive test coverage for line number accuracy
   - Clear error handling for location extraction failures

4. **Backward Compatibility**
   - Existing PerformanceDetector trait remains functional
   - No breaking changes to public APIs
   - Gradual migration path for existing detectors
   - Fallback to standard behavior when location unavailable

## Acceptance Criteria

- [ ] **I/O Detector Line Extraction**: IOPerformanceDetector reports actual line numbers of File::open, read, write operations
- [ ] **String Detector Line Extraction**: StringPerformanceDetector reports actual line numbers of string concatenation, formatting issues
- [ ] **Loop Detector Line Extraction**: NestedLoopDetector reports actual line numbers of nested loop constructs
- [ ] **Allocation Detector Line Extraction**: AllocationDetector reports actual line numbers of problematic allocations
- [ ] **Data Structure Detector Line Extraction**: DataStructureDetector reports actual line numbers of inefficient collection usage
- [ ] **Enhanced Pattern Structure**: All PerformanceAntiPattern variants include source location information
- [ ] **Accurate Debt Items**: convert_performance_pattern_to_debt_item uses extracted line numbers
- [ ] **No False Positives**: No performance issues reported on import statements or non-performance code
- [ ] **Test Coverage**: Comprehensive tests verify line number accuracy for all pattern types
- [ ] **Performance**: Line extraction overhead is <5% of total analysis time

## Technical Details

### Implementation Approach

#### 1. Enhanced Performance Pattern Structure

```rust
// src/performance/mod.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: usize,
    pub column: Option<usize>,
    pub end_line: Option<usize>, // For multi-line patterns
    pub end_column: Option<usize>,
    pub confidence: LocationConfidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LocationConfidence {
    Exact,        // Precise syn::Span information
    Approximate,  // Estimated from surrounding context
    Unavailable,  // No location information available
}

// Updated PerformanceAntiPattern to include location
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PerformanceAntiPattern {
    NestedLoop {
        nesting_level: u32,
        estimated_complexity: ComplexityClass,
        inner_operations: Vec<LoopOperation>,
        can_parallelize: bool,
        location: SourceLocation,  // NEW: Actual source location
    },
    InefficientDataStructure {
        operation: DataStructureOperation,
        collection_type: String,
        recommended_alternative: String,
        performance_impact: PerformanceImpact,
        location: SourceLocation,  // NEW: Actual source location
    },
    ExcessiveAllocation {
        allocation_type: AllocationType,
        frequency: AllocationFrequency,
        suggested_optimization: String,
        location: SourceLocation,  // NEW: Actual source location
    },
    InefficientIO {
        io_pattern: IOPattern,
        batching_opportunity: bool,
        async_opportunity: bool,
        location: SourceLocation,  // NEW: Actual source location
    },
    StringProcessingAntiPattern {
        pattern_type: StringAntiPattern,
        performance_impact: PerformanceImpact,
        recommended_approach: String,
        location: SourceLocation,  // NEW: Actual source location
    },
}

impl PerformanceAntiPattern {
    pub fn location(&self) -> &SourceLocation {
        match self {
            PerformanceAntiPattern::NestedLoop { location, .. } => location,
            PerformanceAntiPattern::InefficientDataStructure { location, .. } => location,
            PerformanceAntiPattern::ExcessiveAllocation { location, .. } => location,
            PerformanceAntiPattern::InefficientIO { location, .. } => location,
            PerformanceAntiPattern::StringProcessingAntiPattern { location, .. } => location,
        }
    }
    
    pub fn primary_line(&self) -> usize {
        self.location().line
    }
}
```

#### 2. Location Extraction Utilities

```rust
// src/performance/location_extractor.rs
use syn::spanned::Spanned;
use syn::{Expr, Stmt, Item};

pub struct LocationExtractor {
    source_lines: Vec<String>,
}

impl LocationExtractor {
    pub fn new(source_content: &str) -> Self {
        Self {
            source_lines: source_content.lines().map(String::from).collect(),
        }
    }
    
    /// Extract location from any syn AST node that implements Spanned
    pub fn extract_location<T: Spanned>(&self, node: &T) -> SourceLocation {
        let span = node.span();
        
        match self.span_to_location(span) {
            Some(location) => location,
            None => SourceLocation {
                line: 1,
                column: None,
                end_line: None,
                end_column: None,
                confidence: LocationConfidence::Unavailable,
            }
        }
    }
    
    /// Extract location from expression with high precision
    pub fn extract_expr_location(&self, expr: &Expr) -> SourceLocation {
        let span = expr.span();
        self.span_to_location(span).unwrap_or_else(|| {
            self.fallback_location_from_context(expr)
        })
    }
    
    /// Extract location from statement
    pub fn extract_stmt_location(&self, stmt: &Stmt) -> SourceLocation {
        let span = stmt.span();
        self.span_to_location(span).unwrap_or_else(|| SourceLocation {
            line: 1,
            column: None,
            end_line: None,
            end_column: None,
            confidence: LocationConfidence::Unavailable,
        })
    }
    
    fn span_to_location(&self, span: proc_macro2::Span) -> Option<SourceLocation> {
        // syn::Span provides line and column information
        let start = span.start();
        let end = span.end();
        
        // syn line numbers are 1-based
        let line = start.line;
        let column = Some(start.column);
        let end_line = if end.line != start.line { Some(end.line) } else { None };
        let end_column = if end.line != start.line || end.column != start.column { 
            Some(end.column) 
        } else { 
            None 
        };
        
        Some(SourceLocation {
            line,
            column,
            end_line,
            end_column,
            confidence: LocationConfidence::Exact,
        })
    }
    
    fn fallback_location_from_context(&self, expr: &Expr) -> SourceLocation {
        // If span information is unavailable, try to estimate from expression type
        // This is a fallback for edge cases where syn spans are not available
        SourceLocation {
            line: 1, // Conservative fallback
            column: None,
            end_line: None,
            end_column: None,
            confidence: LocationConfidence::Unavailable,
        }
    }
}
```

#### 3. Enhanced I/O Performance Detector

```rust
// src/performance/io_detector.rs - Updated implementation
use super::{IOPattern, PerformanceAntiPattern, PerformanceDetector, PerformanceImpact, SourceLocation, LocationConfidence};
use crate::performance::location_extractor::LocationExtractor;
use std::path::Path;
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ExprForLoop, ExprLoop, ExprWhile, File};

pub struct IOPerformanceDetector {
    location_extractor: Option<LocationExtractor>,
}

impl IOPerformanceDetector {
    pub fn new() -> Self {
        Self {
            location_extractor: None,
        }
    }
    
    pub fn with_source_content(source_content: &str) -> Self {
        Self {
            location_extractor: Some(LocationExtractor::new(source_content)),
        }
    }
}

impl PerformanceDetector for IOPerformanceDetector {
    fn detect_anti_patterns(&self, file: &File, path: &Path) -> Vec<PerformanceAntiPattern> {
        // If no location extractor, create one from file content if possible
        let location_extractor = self.location_extractor.as_ref()
            .or_else(|| {
                // Attempt to read source file for location extraction
                std::fs::read_to_string(path).ok()
                    .map(|content| LocationExtractor::new(&content))
                    .as_ref()
            });
            
        let mut visitor = IOVisitor {
            patterns: Vec::new(),
            in_loop: false,
            loop_depth: 0,
            location_extractor,
        };

        visitor.visit_file(file);
        visitor.patterns
    }

    fn detector_name(&self) -> &'static str {
        "IOPerformanceDetector"
    }

    fn estimate_impact(&self, pattern: &PerformanceAntiPattern) -> PerformanceImpact {
        match pattern {
            PerformanceAntiPattern::InefficientIO { io_pattern, .. } => match io_pattern {
                IOPattern::SyncInLoop => PerformanceImpact::High,
                IOPattern::UnbatchedQueries => PerformanceImpact::Critical,
                IOPattern::UnbufferedIO => PerformanceImpact::Medium,
                IOPattern::ExcessiveConnections => PerformanceImpact::High,
            },
            _ => PerformanceImpact::Low,
        }
    }
}

struct IOVisitor<'a> {
    patterns: Vec<PerformanceAntiPattern>,
    in_loop: bool,
    loop_depth: usize,
    location_extractor: Option<&'a LocationExtractor>,
}

impl<'a> IOVisitor<'a> {
    fn check_io_operation(&mut self, expr: &Expr) {
        if !self.in_loop {
            return;
        }

        let location = self.extract_location(expr);

        // Check for file I/O operations
        if let Expr::Call(call) = expr {
            if let Expr::Path(path) = &*call.func {
                let path_str = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");

                if self.is_file_io(&path_str) {
                    self.patterns.push(PerformanceAntiPattern::InefficientIO {
                        io_pattern: IOPattern::SyncInLoop,
                        batching_opportunity: true,
                        async_opportunity: true,
                        location,  // NEW: Actual source location
                    });
                } else if self.is_database_operation(&path_str) {
                    self.patterns.push(PerformanceAntiPattern::InefficientIO {
                        io_pattern: IOPattern::UnbatchedQueries,
                        batching_opportunity: true,
                        async_opportunity: true,
                        location,  // NEW: Actual source location
                    });
                } else if self.is_network_operation(&path_str) {
                    self.patterns.push(PerformanceAntiPattern::InefficientIO {
                        io_pattern: IOPattern::SyncInLoop,
                        batching_opportunity: false,
                        async_opportunity: true,
                        location,  // NEW: Actual source location
                    });
                }
            }
        }

        // Check for method calls that might be I/O
        if let Expr::MethodCall(method_call) = expr {
            let method_name = method_call.method.to_string();

            if self.is_io_method(&method_name) {
                let (io_pattern, can_batch) =
                    if method_name.contains("query") || method_name.contains("execute") {
                        (IOPattern::UnbatchedQueries, true)
                    } else {
                        (IOPattern::SyncInLoop, false)
                    };

                self.patterns.push(PerformanceAntiPattern::InefficientIO {
                    io_pattern,
                    batching_opportunity: can_batch,
                    async_opportunity: true,
                    location,  // NEW: Actual source location
                });
            }
        }
    }
    
    fn extract_location(&self, expr: &Expr) -> SourceLocation {
        if let Some(extractor) = self.location_extractor {
            extractor.extract_expr_location(expr)
        } else {
            // Fallback when no source content available
            SourceLocation {
                line: 1,
                column: None,
                end_line: None,
                end_column: None,
                confidence: LocationConfidence::Unavailable,
            }
        }
    }
    
    fn check_unbuffered_io(&mut self, call: &ExprCall) {
        if let Expr::Path(path) = &*call.func {
            let path_str = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            // Check for direct file operations without buffering
            if path_str.contains("File::open") || path_str.contains("File::create") {
                let location = self.extract_location(&Expr::Call(call.clone()));
                
                self.patterns.push(PerformanceAntiPattern::InefficientIO {
                    io_pattern: IOPattern::UnbufferedIO,
                    batching_opportunity: false,
                    async_opportunity: false,
                    location,  // NEW: Actual source location
                });
            }
        }
    }

    // ... existing methods for pattern recognition remain unchanged
}
```

#### 4. Updated Debt Item Conversion

```rust
// src/performance/mod.rs - Updated conversion function
pub fn convert_performance_pattern_to_debt_item(
    pattern: PerformanceAntiPattern,
    impact: PerformanceImpact,
    path: &Path,
) -> DebtItem {
    let location = pattern.location();
    let line = location.line;
    
    let priority = match &pattern {
        PerformanceAntiPattern::NestedLoop {
            estimated_complexity,
            ..
        } => classify_nested_loop_priority(estimated_complexity),
        PerformanceAntiPattern::InefficientIO { .. } => Priority::High,
        _ => impact_to_priority(impact),
    };

    let message = format_pattern_message(&pattern);
    let recommendation = generate_pattern_recommendation(&pattern);

    DebtItem {
        id: format!("performance-{}-{}", path.display(), line),
        debt_type: DebtType::Performance,
        priority,
        file: path.to_path_buf(),
        line,  // NOW: Uses actual extracted line number
        column: location.column,  // NEW: Optional column information
        message,
        context: Some(format!("{}\nLocation confidence: {:?}", 
                             recommendation, 
                             location.confidence)),
    }
}
```

#### 5. Restored Analyzer Integration

```rust
// src/analyzers/rust.rs - Fixed performance pattern analysis
fn analyze_performance_patterns(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    // Read source content for accurate line extraction
    let source_content = std::fs::read_to_string(path).unwrap_or_default();
    
    let detectors: Vec<Box<dyn PerformanceDetector>> = vec![
        Box::new(NestedLoopDetector::with_source_content(&source_content)),
        Box::new(DataStructureDetector::with_source_content(&source_content)),
        Box::new(AllocationDetector::with_source_content(&source_content)),
        Box::new(IOPerformanceDetector::with_source_content(&source_content)),
        Box::new(StringPerformanceDetector::with_source_content(&source_content)),
    ];

    let mut performance_items = Vec::new();

    for detector in detectors {
        let anti_patterns = detector.detect_anti_patterns(file, path);

        for pattern in anti_patterns {
            let impact = detector.estimate_impact(&pattern);
            // NOW: Uses actual line numbers from pattern location
            let debt_item = convert_performance_pattern_to_debt_item(pattern, impact, path);
            performance_items.push(debt_item);
        }
    }

    performance_items
}
```

### Architecture Changes

#### Modified Components
- `src/performance/mod.rs`: Enhanced PerformanceAntiPattern with SourceLocation
- `src/performance/location_extractor.rs`: New utility for extracting line numbers from syn spans
- `src/performance/io_detector.rs`: Updated to extract actual I/O operation locations
- `src/performance/nested_loop_detector.rs`: Updated to extract actual loop locations
- `src/performance/string_detector.rs`: Updated to extract actual string operation locations
- `src/performance/allocation_detector.rs`: Updated to extract actual allocation locations
- `src/performance/data_structure_detector.rs`: Updated to extract actual collection operation locations
- `src/analyzers/rust.rs`: Restored performance pattern analysis with accurate line numbers

#### New Components
- `SourceLocation` struct for precise location tracking
- `LocationConfidence` enum for location accuracy indication
- `LocationExtractor` utility for consistent span-to-location conversion
- Enhanced test fixtures with known line numbers for validation

### Data Structures

#### Enhanced Performance Pattern
```rust
// Before: No location information
PerformanceAntiPattern::InefficientIO {
    io_pattern: IOPattern::SyncInLoop,
    batching_opportunity: true,
    async_opportunity: true,
}

// After: With precise location
PerformanceAntiPattern::InefficientIO {
    io_pattern: IOPattern::SyncInLoop,
    batching_opportunity: true,
    async_opportunity: true,
    location: SourceLocation {
        line: 42,
        column: Some(16),
        end_line: None,
        end_column: None,
        confidence: LocationConfidence::Exact,
    },
}
```

#### Enhanced Debt Item
```rust
// Before: Arbitrary line number
DebtItem {
    id: "performance-test.rs-1",
    line: 1,  // ← Wrong: import statement
    message: "Inefficient I/O pattern: SyncInLoop",
    ...
}

// After: Actual source location
DebtItem {
    id: "performance-test.rs-42",
    line: 42,  // ← Correct: actual I/O operation
    column: Some(16),
    message: "Inefficient I/O pattern: SyncInLoop",
    context: Some("Consider: batch operations\nLocation confidence: Exact"),
    ...
}
```

### Integration Points

#### Source Content Integration
```rust
// Analyzers provide source content to detectors
let source_content = std::fs::read_to_string(path)?;
let detector = IOPerformanceDetector::with_source_content(&source_content);
```

#### Fallback Handling
```rust
// Graceful degradation when source unavailable
let detector = IOPerformanceDetector::new(); // Works without source
let patterns = detector.detect_anti_patterns(file, path);
// Patterns will have LocationConfidence::Unavailable
```

## Dependencies

### Prerequisites
- No specification dependencies
- Uses existing syn, anyhow crates
- Leverages existing PerformanceDetector trait architecture

### Affected Components
- `src/performance/`: All performance detectors require location extraction updates
- `src/analyzers/rust.rs`: Performance pattern analysis integration
- Performance-related tests: Need updates for line number verification

### External Dependencies
- No new external dependencies required
- Uses existing syn::spanned::Spanned trait for location extraction
- Leverages proc_macro2::Span for line/column information

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_io_detector_accurate_line_numbers() {
        let source = r#"
use std::fs::File;
use std::io::Read;

fn process_files(paths: &[String]) -> Vec<String> {
    let mut results = Vec::new();
    for path in paths {                           // Line 7
        let mut file = File::open(path).unwrap(); // Line 8 ← Should be detected here
        let mut contents = String::new();
        file.read_to_string(&mut contents);       // Line 10 ← Should be detected here
        results.push(contents);
    }
    results
}
        "#;

        let file = syn::parse_str::<syn::File>(source).unwrap();
        let detector = IOPerformanceDetector::with_source_content(source);
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        assert!(!patterns.is_empty(), "Should detect I/O patterns in loop");
        
        // Verify line numbers are accurate
        let line_numbers: Vec<usize> = patterns.iter()
            .map(|p| p.primary_line())
            .collect();
        
        // Should detect I/O operations on lines 8 and 10, NOT on import lines 1-2
        assert!(line_numbers.contains(&8), "Should detect File::open on line 8");
        assert!(line_numbers.contains(&10), "Should detect read_to_string on line 10");
        assert!(!line_numbers.contains(&1), "Should NOT detect on import line 1");
        assert!(!line_numbers.contains(&2), "Should NOT detect on import line 2");
    }

    #[test]
    fn test_location_confidence_levels() {
        let source = "fn test() { std::fs::read_to_string(\"file\").unwrap(); }";
        
        let file = syn::parse_str::<syn::File>(source).unwrap();
        let detector = IOPerformanceDetector::with_source_content(source);
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        if let Some(pattern) = patterns.first() {
            assert_eq!(pattern.location().confidence, LocationConfidence::Exact);
            assert_eq!(pattern.location().line, 1);
            assert!(pattern.location().column.is_some());
        }
    }

    #[test]
    fn test_fallback_without_source_content() {
        let source = "fn test() { std::fs::read_to_string(\"file\").unwrap(); }";
        
        let file = syn::parse_str::<syn::File>(source).unwrap();
        let detector = IOPerformanceDetector::new(); // No source content
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        // Should still detect patterns but with unavailable location
        if let Some(pattern) = patterns.first() {
            assert_eq!(pattern.location().confidence, LocationConfidence::Unavailable);
        }
    }

    #[test]
    fn test_nested_loop_line_detection() {
        let source = r#"
fn matrix_multiply(a: &[Vec<i32>], b: &[Vec<i32>]) -> Vec<Vec<i32>> {
    let mut result = vec![vec![0; b[0].len()]; a.len()];
    for i in 0..a.len() {        // Line 4
        for j in 0..b[0].len() { // Line 5 ← Should be detected here
            for k in 0..b.len() { // Line 6 ← Should be detected here
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}
        "#;

        let file = syn::parse_str::<syn::File>(source).unwrap();
        let detector = NestedLoopDetector::with_source_content(source);
        let patterns = detector.detect_anti_patterns(&file, Path::new("test.rs"));

        assert!(!patterns.is_empty(), "Should detect nested loops");
        
        let line_numbers: Vec<usize> = patterns.iter()
            .map(|p| p.primary_line())
            .collect();
        
        // Should detect nested loop pattern starting from innermost or middle loop
        assert!(line_numbers.iter().any(|&line| line >= 5 && line <= 6),
               "Should detect nested loops on lines 5-6, got: {:?}", line_numbers);
    }
}
```

### Integration Tests

```rust
// tests/performance_line_extraction_integration.rs
use std::process::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_end_to_end_performance_line_accuracy() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    
    // Create test file with known performance issues at specific lines
    fs::write(&test_file, r#"
use std::fs::File;        // Line 2 - import (should NOT be flagged)
use std::io::Read;        // Line 3 - import (should NOT be flagged)

fn inefficient_io(files: &[String]) -> Vec<String> {
    let mut contents = Vec::new();
    for file_path in files {                              // Line 7
        let mut file = File::open(file_path).unwrap();    // Line 8 ← SHOULD be flagged
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();       // Line 10 ← SHOULD be flagged
        contents.push(content);
    }
    contents
}
    "#).unwrap();

    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", test_file.to_str().unwrap(), "--performance-only", "--detailed"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Verify that performance issues are reported on correct lines
    assert!(stdout.contains(":8"), "Should report performance issue on line 8");
    assert!(stdout.contains(":10"), "Should report performance issue on line 10");
    
    // Verify that import lines are NOT flagged
    assert!(!stdout.contains(":2"), "Should NOT report performance issue on import line 2");
    assert!(!stdout.contains(":3"), "Should NOT report performance issue on import line 3");
    
    // Verify performance patterns are detected
    assert!(stdout.contains("Blocking I/O") || stdout.contains("SyncInLoop"), 
           "Should detect I/O performance patterns");
}

#[test]
fn test_json_output_includes_line_numbers() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    
    fs::write(&test_file, r#"
fn string_concat_in_loop(items: &[String]) -> String {
    let mut result = String::new();
    for item in items {                    // Line 4
        result = result + item;            // Line 5 ← Should be flagged
    }
    result
}
    "#).unwrap();

    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", test_file.to_str().unwrap(), "--performance-only", "--format", "json"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    
    // Find performance debt items
    let debt_items = json["analysis"]["debt_items"].as_array().unwrap();
    let performance_items: Vec<_> = debt_items.iter()
        .filter(|item| item["debt_type"] == "Performance")
        .collect();
    
    assert!(!performance_items.is_empty(), "Should have performance debt items");
    
    // Verify line numbers are accurate
    for item in performance_items {
        let line = item["line"].as_u64().unwrap();
        assert!(line >= 4, "Performance issue should be on line 4 or later, got line {}", line);
        assert!(line != 1, "Should not flag line 1");
        assert!(line != 2, "Should not flag line 2");
    }
}
```

### Performance Tests

```rust
#[test]
fn test_line_extraction_performance() {
    use std::time::Instant;
    
    // Test with large file to ensure line extraction doesn't add significant overhead
    let large_source = generate_large_rust_file_with_patterns(1000); // 1000 functions with various patterns
    
    let start = Instant::now();
    let file = syn::parse_str::<syn::File>(&large_source).unwrap();
    let parse_time = start.elapsed();
    
    let start = Instant::now();
    let detector = IOPerformanceDetector::with_source_content(&large_source);
    let patterns = detector.detect_anti_patterns(&file, Path::new("large_test.rs"));
    let detection_time = start.elapsed();
    
    let start = Instant::now();
    let detector_no_source = IOPerformanceDetector::new();
    let patterns_no_source = detector_no_source.detect_anti_patterns(&file, Path::new("large_test.rs"));
    let detection_time_no_source = start.elapsed();
    
    // Line extraction should add <5% overhead
    let overhead_ratio = detection_time.as_nanos() as f64 / detection_time_no_source.as_nanos() as f64;
    assert!(overhead_ratio < 1.05, "Line extraction overhead too high: {:.2}%", (overhead_ratio - 1.0) * 100.0);
    
    // Verify patterns were detected
    assert!(!patterns.is_empty(), "Should detect patterns in large file");
    assert_eq!(patterns.len(), patterns_no_source.len(), "Should detect same number of patterns");
}
```

## Documentation Requirements

### Code Documentation
- Comprehensive rustdoc for SourceLocation and LocationExtractor
- Examples of line number extraction for each pattern type
- Performance characteristics and accuracy limitations
- Migration guide for updating existing performance detectors

### User Documentation
```markdown
## Performance Analysis Accuracy

Debtmap now provides precise source locations for all performance issues:

### Line Number Accuracy
- **Exact locations**: Performance issues are reported at their actual source lines
- **No false positives**: Import statements and non-performance code are not flagged
- **Multi-line patterns**: Complex patterns report their primary location with optional ranges
- **Confidence indicators**: Location accuracy is indicated in detailed output

### Before/After Comparison
```bash
# Before: False positives on import lines
#1 SCORE: 8.3 [CRITICAL]
├─ PERFORMANCE: test.rs:1 performance_issue_at_line_1()  # ← Wrong: import line
└─ WHY: Performance issue (High) detected: Blocking I/O

# After: Accurate source locations  
#1 SCORE: 8.3 [CRITICAL]
├─ PERFORMANCE: test.rs:42 blocking_io_in_loop()       # ← Correct: actual I/O operation
└─ WHY: Performance issue (High) detected: Blocking I/O at File::open call
```

### Output Format
Performance issues now include precise location information:
```
📊 PERFORMANCE ANALYSIS
├─ Issue: Blocking I/O in loop
├─ Location: src/file_processor.rs:42:16
├─ Confidence: Exact
└─ Context: File::open call within iterator loop
```
```

## Implementation Notes

### Phased Implementation
1. **Phase 1**: Enhanced PerformanceAntiPattern structure with SourceLocation
2. **Phase 2**: LocationExtractor utility and basic line extraction
3. **Phase 3**: Updated IOPerformanceDetector with accurate line reporting
4. **Phase 4**: Updated remaining detectors (String, Loop, Allocation, DataStructure)
5. **Phase 5**: Integration testing and performance validation

### Edge Cases to Consider
- **Macro-expanded code**: Use syn span information, fallback to original source locations
- **Missing source files**: Graceful degradation with LocationConfidence::Unavailable
- **Multi-line patterns**: Report primary line with optional range information
- **Complex expressions**: Extract location from most relevant sub-expression

### Migration Strategy
- **Backward compatibility**: Existing PerformanceDetector implementations continue working
- **Gradual adoption**: Detectors can be updated incrementally
- **Fallback behavior**: Missing source content results in unavailable location confidence
- **Test validation**: Comprehensive tests ensure line number accuracy

## Expected Impact

After implementation:

1. **Eliminated False Positives**: No more performance issues on import statements or non-performance code
2. **Accurate Debugging**: Developers can immediately locate actual performance problems in their code  
3. **Restored Confidence**: Performance analysis becomes trustworthy and actionable
4. **Enhanced Workflow**: Precise source locations enable direct code navigation and fixes
5. **Complete Coverage**: All performance pattern types report accurate source locations

## Migration and Compatibility

- **Breaking Changes**: None - enhanced location information is additive
- **API Compatibility**: Existing PerformanceDetector trait remains unchanged
- **Output Enhancement**: Reports include precise location information without breaking existing parsers
- **Configuration**: No new configuration required - line extraction works automatically
- **Performance**: <5% overhead for enhanced location accuracy

This fix resolves the critical false positive issue in performance analysis while maintaining full backward compatibility and providing significant accuracy improvements for performance debugging workflows.