---
number: 37
title: Unified Detector Line Number Extraction
category: optimization
priority: critical
status: draft
dependencies: [36]
created: 2025-08-17
---

# Specification 37: Unified Detector Line Number Extraction

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: [36 - Performance Detector Line Number Extraction]

## Context

A systematic evaluation of all detector integrations has revealed widespread **critical line number bugs** affecting multiple detector categories beyond just performance patterns. These bugs significantly impact tool usability by making it impossible for developers to locate the actual source of detected issues.

### Current Line Number Issues

#### **Organization Detectors** (CRITICAL)
- **Location**: `src/analyzers/rust.rs:782`
- **Issue**: `line: 0,` hardcoded for all organization patterns
- **Impact**: All organization issues (GodObject, MagicValue, LongParameterList, FeatureEnvy, PrimitiveObsession, DataClump) attributed to non-existent line 0

#### **Security Detectors** (CRITICAL) 
- **Input Validation**: `src/security/input_validation_detector.rs:117` - `check_function_validation(0)`
- **Hardcoded Secrets**: `src/security/hardcoded_secret_detector.rs:117` - `check_string_for_secrets(&value, 0)`
- **Enhanced Detectors**: Multiple hardcoded `line: 0` in:
  - `enhanced_secret_detector.rs`: Lines 257, 280
  - `enhanced_sql_detector.rs`: Lines 224, 245, 265, 284  
  - `taint_analysis.rs`: Lines 123, 259, 270, 302, 330

#### **Resource Detectors** (MEDIUM)
- **Location**: `src/resource/mod.rs:247`
- **Issue**: `_ => 0,` fallback for issues without specific line numbers
- **Impact**: Most resource issues (MissingDrop, AsyncResourceIssue, UnboundedCollection, RaiiViolation) attributed to line 0

#### **Performance Detectors** (FIXED)
- **Status**: Temporarily disabled due to arbitrary line assignment bug
- **Dependency**: Spec 36 addresses comprehensive performance detector line extraction

#### **Testing Detectors** (WORKING)
- **Status**: Already include proper line numbers in TestingAntiPattern structure
- **Implementation**: Correctly extract line information from AST

### Impact Assessment

This is a **systemic problem** affecting the majority of debt detection capabilities:

- **Debugging Impossible**: Line 0 reports provide no guidance for locating actual issues
- **Tool Credibility**: Makes debtmap appear unreliable and poorly implemented
- **Development Friction**: Forces manual code search to find reported problems
- **False Dismissal**: Developers may ignore valid issues due to unusable location information

The line number extraction problem represents a fundamental architectural flaw in detector integration that undermines the core value proposition of the tool.

## Objective

Implement a unified line number extraction system that provides accurate source locations for all detector categories, building on the foundation established in Spec 36 for performance detectors.

**Goals:**
1. **Accurate Source Locations**: All detected issues report their actual source line numbers
2. **Unified Architecture**: Consistent line extraction approach across all detector types
3. **Backward Compatibility**: No breaking changes to existing detector interfaces
4. **Performance Efficiency**: Minimal overhead for enhanced location accuracy
5. **Comprehensive Coverage**: Address organization, security, and resource detector line number issues

## Requirements

### Functional Requirements

1. **Organization Pattern Line Extraction**
   - Extract actual line numbers for GodObject, MagicValue, LongParameterList, FeatureEnvy, PrimitiveObsession, and DataClump patterns
   - Support both struct/type definitions and function-based patterns
   - Handle multi-line patterns by reporting primary location

2. **Security Pattern Line Extraction**
   - Extract line numbers for input validation gaps at function declaration sites
   - Extract line numbers for hardcoded secrets at literal expression locations
   - Extract line numbers for SQL injection risks at query construction sites
   - Support enhanced detector line extraction using taint analysis paths

3. **Resource Pattern Line Extraction**  
   - Extract line numbers for missing Drop implementations at struct definition sites
   - Extract line numbers for resource leaks at allocation/acquisition sites
   - Extract line numbers for async resource issues at async function sites
   - Extract line numbers for unbounded collections at collection declaration sites

4. **Unified Location Infrastructure**
   - Extend SourceLocation structure from Spec 36 for use across all detector types
   - Provide LocationExtractor utility for consistent syn::Span-based extraction
   - Support confidence levels (Exact, Approximate, Unavailable) for all detectors
   - Enable graceful fallback when source information is unavailable

### Non-Functional Requirements

1. **Performance**
   - Line extraction adds <10% overhead to total analysis time
   - Efficient span information retrieval across all detector types
   - Minimal memory overhead for storing enhanced location information
   - No impact on existing analysis workflows

2. **Accuracy**
   - 100% accuracy for extractable line numbers from syn spans
   - Clear indication when line information is uncertain or unavailable
   - Consistent location reporting format across all detector types
   - No false line number assignments

3. **Maintainability**
   - Unified location extraction patterns reduce code duplication
   - Clear separation between pattern detection and location extraction
   - Comprehensive test coverage for line number accuracy across all detectors
   - Consistent error handling for location extraction failures

4. **Simplicity**
   - Breaking changes acceptable as we're still in prototype phase
   - Direct implementation without legacy compatibility concerns
   - Clean, straightforward detector interface redesign
   - Immediate adoption of enhanced location tracking

## Acceptance Criteria

- [ ] **Organization Pattern Accuracy**: All organization patterns report actual source line numbers instead of hardcoded 0
- [ ] **Security Pattern Accuracy**: All security patterns report actual source line numbers for validation gaps, secrets, and SQL injection risks  
- [ ] **Resource Pattern Accuracy**: All resource patterns report actual source line numbers for Drop issues, leaks, and async problems
- [ ] **Unified Location Structure**: SourceLocation structure used consistently across organization, security, and resource detectors
- [ ] **Location Confidence**: All detectors provide confidence levels for reported line numbers
- [ ] **No False Line Numbers**: Zero instances of hardcoded line 0 or arbitrary line assignments
- [ ] **Simplified Implementation**: Clean detector interfaces without legacy compatibility concerns
- [ ] **Performance**: Line extraction overhead is <10% of total analysis time
- [ ] **Test Coverage**: Comprehensive integration tests verify line number accuracy for all detector categories
- [ ] **Documentation**: Updated architecture documentation reflects unified line extraction approach

## Technical Details

### Implementation Approach

#### 1. Enhanced Pattern Structures with Unified Location

Building on Spec 36's SourceLocation, extend all pattern enums to include location information:

```rust
// src/common/source_location.rs - Shared location utilities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: usize,
    pub column: Option<usize>,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
    pub confidence: LocationConfidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LocationConfidence {
    Exact,        // Precise syn::Span information
    Approximate,  // Estimated from surrounding context
    Unavailable,  // No location information available
}

// Common utilities for all detectors
pub struct UnifiedLocationExtractor {
    source_lines: Vec<String>,
}

impl UnifiedLocationExtractor {
    pub fn new(source_content: &str) -> Self {
        Self {
            source_lines: source_content.lines().map(String::from).collect(),
        }
    }
    
    /// Extract location from any syn AST node that implements Spanned
    pub fn extract_location<T: Spanned>(&self, node: &T) -> SourceLocation {
        // Implementation from Spec 36, enhanced for broader usage
        let span = node.span();
        self.span_to_location(span).unwrap_or_else(|| SourceLocation {
            line: 1,
            column: None,
            end_line: None,
            end_column: None,
            confidence: LocationConfidence::Unavailable,
        })
    }
    
    /// Extract location from item definitions (structs, enums, functions)
    pub fn extract_item_location(&self, item: &syn::Item) -> SourceLocation {
        self.extract_location(item)
    }
    
    /// Extract location from expressions
    pub fn extract_expr_location(&self, expr: &syn::Expr) -> SourceLocation {
        self.extract_location(expr)
    }
    
    /// Extract location from type definitions
    pub fn extract_type_location(&self, ty: &syn::Type) -> SourceLocation {
        self.extract_location(ty)
    }
}
```

#### 2. Enhanced Organization Pattern Structure

```rust
// src/organization/mod.rs - Updated organization patterns
#[derive(Debug, Clone, PartialEq)]
pub enum OrganizationAntiPattern {
    GodObject {
        type_name: String,
        method_count: usize,
        field_count: usize,
        suggested_split: Vec<ResponsibilityGroup>,
        location: SourceLocation,  // NEW: Actual struct definition location
    },
    MagicValue {
        value: String,
        occurrence_count: usize,
        suggested_constant_name: String,
        locations: Vec<SourceLocation>,  // NEW: All occurrence locations
    },
    LongParameterList {
        function_name: String,
        parameter_count: usize,
        suggested_refactoring: RefactoringStrategy,
        location: SourceLocation,  // NEW: Function signature location
    },
    FeatureEnvy {
        method_name: String,
        envied_type: String,
        external_calls: usize,
        internal_calls: usize,
        location: SourceLocation,  // NEW: Method definition location
    },
    PrimitiveObsession {
        primitive_type: String,
        usage_context: UsageContext,
        suggested_domain_type: String,
        locations: Vec<SourceLocation>,  // NEW: All usage locations
    },
    DataClump {
        parameter_group: ParameterGroup,
        suggested_struct_name: String,
        locations: Vec<SourceLocation>,  // NEW: All function signature locations
    },
}

impl OrganizationAntiPattern {
    pub fn primary_location(&self) -> &SourceLocation {
        match self {
            OrganizationAntiPattern::GodObject { location, .. } => location,
            OrganizationAntiPattern::MagicValue { locations, .. } => &locations[0],
            OrganizationAntiPattern::LongParameterList { location, .. } => location,
            OrganizationAntiPattern::FeatureEnvy { location, .. } => location,
            OrganizationAntiPattern::PrimitiveObsession { locations, .. } => &locations[0],
            OrganizationAntiPattern::DataClump { locations, .. } => &locations[0],
        }
    }
    
    pub fn all_locations(&self) -> Vec<&SourceLocation> {
        match self {
            OrganizationAntiPattern::GodObject { location, .. } => vec![location],
            OrganizationAntiPattern::MagicValue { locations, .. } => locations.iter().collect(),
            OrganizationAntiPattern::LongParameterList { location, .. } => vec![location],
            OrganizationAntiPattern::FeatureEnvy { location, .. } => vec![location],
            OrganizationAntiPattern::PrimitiveObsession { locations, .. } => locations.iter().collect(),
            OrganizationAntiPattern::DataClump { locations, .. } => locations.iter().collect(),
        }
    }
}
```

#### 3. Enhanced Security Pattern Integration

```rust
// src/security/types.rs - Enhanced security vulnerabilities with location
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityVulnerability {
    HardcodedSecret {
        secret_type: SecretType,
        confidence: f64,
        entropy: f64,
        line: usize,           // EXISTING: Keep for backward compatibility
        file: PathBuf,         // EXISTING: Keep for backward compatibility
        location: SourceLocation,  // NEW: Enhanced location information
        pattern_matched: String,
    },
    SqlInjection {
        injection_type: SqlInjectionType,
        taint_source: Option<TaintSource>,
        severity: Severity,
        line: usize,           // EXISTING: Keep for backward compatibility
        file: PathBuf,         // EXISTING: Keep for backward compatibility
        location: SourceLocation,  // NEW: Enhanced location information
        query_context: String,
    },
    InputValidationGap {
        input_source: InputSource,
        sink_operation: SinkOperation,
        taint_path: TaintPath,
        severity: Severity,
        line: usize,           // EXISTING: Keep for backward compatibility
        file: PathBuf,         // EXISTING: Keep for backward compatibility
        location: SourceLocation,  // NEW: Enhanced location information
        function_signature: String,
    },
    // ... other variants with similar location enhancement
}
```

#### 4. Updated Organization Detector Implementation

```rust
// src/organization/god_object_detector.rs - Enhanced with line extraction
use crate::common::source_location::{SourceLocation, UnifiedLocationExtractor};

pub struct GodObjectDetector {
    location_extractor: UnifiedLocationExtractor,
}

impl GodObjectDetector {
    pub fn new(source_content: &str) -> Self {
        Self {
            location_extractor: UnifiedLocationExtractor::new(source_content),
        }
    }
}

impl OrganizationDetector for GodObjectDetector {
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern> {
        let mut patterns = Vec::new();
        
        for item in &file.items {
            if let syn::Item::Struct(struct_item) = item {
                let method_count = self.count_impl_methods(file, &struct_item.ident);
                let field_count = struct_item.fields.len();
                
                if method_count > 10 || field_count > 8 {
                    let location = self.extract_location(struct_item);
                    let suggested_split = self.suggest_responsibility_split(struct_item, method_count);
                    
                    patterns.push(OrganizationAntiPattern::GodObject {
                        type_name: struct_item.ident.to_string(),
                        method_count,
                        field_count,
                        suggested_split,
                        location,  // NOW: Actual struct definition location
                    });
                }
            }
        }
        
        patterns
    }
}

impl GodObjectDetector {
    fn extract_location(&self, struct_item: &syn::ItemStruct) -> SourceLocation {
        self.location_extractor.extract_item_location(&syn::Item::Struct(struct_item.clone()))
    }
}
```

#### 5. Updated Security Detector Implementation

```rust
// src/security/input_validation_detector.rs - Fixed line extraction
use crate::common::source_location::{SourceLocation, UnifiedLocationExtractor, LocationConfidence};

pub struct ValidationVisitor {
    path: std::path::PathBuf,
    debt_items: Vec<DebtItem>,
    current_function: Option<String>,
    current_function_location: Option<SourceLocation>,  // NEW: Track function location
    has_validation: bool,
    has_external_input: bool,
    location_extractor: UnifiedLocationExtractor,  // NEW: Location extraction
}

impl ValidationVisitor {
    fn new(path: &Path, source_content: &str) -> Self {
        Self {
            path: path.to_path_buf(),
            debt_items: Vec::new(),
            current_function: None,
            current_function_location: None,
            has_validation: false,
            has_external_input: false,
            location_extractor: UnifiedLocationExtractor::new(source_content),
        }
    }

    fn check_function_validation(&mut self) {
        if self.has_external_input && !self.has_validation {
            if let Some(ref func_name) = self.current_function {
                let location = self.current_function_location.clone().unwrap_or_else(|| SourceLocation {
                    line: 1,
                    column: None,
                    end_line: None,
                    end_column: None,
                    confidence: LocationConfidence::Unavailable,
                });
                
                self.debt_items.push(DebtItem {
                    id: format!("security-validation-{}-{}", self.path.display(), location.line),
                    debt_type: DebtType::Security,
                    priority: Priority::High,
                    file: self.path.clone(),
                    line: location.line,  // NOW: Uses actual function line
                    column: location.column,
                    message: format!("Missing input validation in function '{}'", func_name),
                    context: Some("External input should be validated before use".to_string()),
                });
            }
        }
    }
}

impl<'ast> Visit<'ast> for ValidationVisitor {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        let prev_function = self.current_function.clone();
        let prev_location = self.current_function_location.clone();
        let prev_validation = self.has_validation;
        let prev_input = self.has_external_input;

        self.current_function = Some(i.sig.ident.to_string());
        
        // NEW: Extract actual function location
        self.current_function_location = Some(
            self.location_extractor.extract_item_location(&syn::Item::Fn(i.clone()))
        );
        
        self.has_validation = false;
        self.has_external_input = false;

        // Check function parameters for external input
        for input in &i.sig.inputs {
            if let syn::FnArg::Typed(pat_type) = input {
                if let Pat::Ident(PatIdent { ident, .. }) = &*pat_type.pat {
                    if self.is_external_input_source(&ident.to_string()) {
                        self.has_external_input = true;
                    }
                }
            }
        }

        // Check if function name suggests it handles external input
        if self.is_external_input_source(&i.sig.ident.to_string()) {
            self.has_external_input = true;
        }

        syn::visit::visit_item_fn(self, i);

        // Check after visiting the function body
        self.check_function_validation();  // NOW: Uses self.current_function_location

        // Restore previous state
        self.current_function = prev_function;
        self.current_function_location = prev_location;
        self.has_validation = prev_validation;
        self.has_external_input = prev_input;
    }
}

pub fn detect_validation_gaps(file: &File, path: &Path, source_content: &str) -> Vec<DebtItem> {
    let mut visitor = ValidationVisitor::new(path, source_content);
    visitor.visit_file(file);
    visitor.debt_items
}
```

#### 6. Updated Resource Pattern Integration

```rust
// src/resource/mod.rs - Enhanced resource issue with location
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceManagementIssue {
    MissingDrop {
        type_name: String,
        resource_fields: Vec<ResourceField>,
        suggested_drop_impl: String,
        severity: ResourceSeverity,
        location: SourceLocation,  // NEW: Struct definition location
    },
    ResourceLeak {
        resource_type: ResourceType,
        acquisition_site: SourceLocation,  // EXISTING
        leak_site: SourceLocation,         // EXISTING
        cleanup_suggestion: String,
        // Enhanced with better location tracking
    },
    AsyncResourceIssue {
        function_name: String,
        issue_type: AsyncResourceIssueType,
        cancellation_safety: CancellationSafety,
        mitigation_strategy: String,
        location: SourceLocation,  // NEW: Async function location
    },
    UnboundedCollection {
        collection_name: String,
        collection_type: String,
        growth_pattern: GrowthPattern,
        bounding_strategy: BoundingStrategy,
        location: SourceLocation,  // NEW: Collection declaration location
    },
    // ... other variants with enhanced location information
}

fn get_line_from_issue(issue: &ResourceManagementIssue) -> usize {
    match issue {
        ResourceManagementIssue::ResourceLeak { leak_site, .. } => leak_site.line,
        ResourceManagementIssue::HandleLeak { leak_location, .. } => leak_location.line,
        ResourceManagementIssue::MissingDrop { location, .. } => location.line,  // NEW
        ResourceManagementIssue::AsyncResourceIssue { location, .. } => location.line,  // NEW
        ResourceManagementIssue::UnboundedCollection { location, .. } => location.line,  // NEW
        ResourceManagementIssue::RaiiViolation { .. } => 1,  // TODO: Add location extraction
    }
}
```

#### 7. Updated Analyzer Integration

```rust
// src/analyzers/rust.rs - Fixed organization pattern conversion
fn convert_organization_pattern_to_debt_item(
    pattern: OrganizationAntiPattern,
    impact: MaintainabilityImpact,
    path: &Path,
) -> DebtItem {
    let location = pattern.primary_location();
    let line = location.line;
    
    let (priority, message, context) = match pattern {
        OrganizationAntiPattern::GodObject {
            type_name,
            method_count,
            field_count,
            suggested_split,
            ..
        } => (
            match impact {
                MaintainabilityImpact::Critical => Priority::Critical,
                MaintainabilityImpact::High => Priority::High,
                MaintainabilityImpact::Medium => Priority::Medium,
                MaintainabilityImpact::Low => Priority::Low,
            },
            format!(
                "God object '{}' with {} methods and {} fields",
                type_name, method_count, field_count
            ),
            Some(format!(
                "Consider splitting into: {}\nLocation confidence: {:?}",
                suggested_split
                    .iter()
                    .map(|g| &g.name)
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                location.confidence
            )),
        ),
        // ... handle other pattern types with actual locations
    };

    DebtItem {
        id: format!("organization-{}-{}", path.display(), line),
        debt_type: DebtType::CodeOrganization,
        priority,
        file: path.to_path_buf(),
        line,  // NOW: Uses actual extracted line number
        column: location.column,  // NEW: Optional column information
        message,
        context,
    }
}

fn analyze_organization_patterns(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    // Read source content for accurate line extraction
    let source_content = std::fs::read_to_string(path).expect("Failed to read source file");
    
    let detectors: Vec<Box<dyn OrganizationDetector>> = vec![
        Box::new(GodObjectDetector::new(&source_content)),
        Box::new(MagicValueDetector::new(&source_content)),
        Box::new(ParameterAnalyzer::new(&source_content)),
        Box::new(FeatureEnvyDetector::new(&source_content)),
        Box::new(PrimitiveObsessionDetector::new(&source_content)),
    ];

    let mut organization_items = Vec::new();

    for detector in detectors {
        let anti_patterns = detector.detect_anti_patterns(file);

        for pattern in anti_patterns {
            let impact = detector.estimate_maintainability_impact(&pattern);
            let debt_item = convert_organization_pattern_to_debt_item(pattern, impact, path);
            organization_items.push(debt_item);
        }
    }

    organization_items
}
```

### Architecture Changes

#### New Components
- `src/common/source_location.rs`: Unified location utilities shared across all detectors
- Enhanced pattern structures in each detector module with SourceLocation fields
- LocationExtractor trait implementations for consistent span extraction
- Updated conversion functions that use actual line numbers

#### Modified Components
- `src/organization/`: All organization detectors updated with line extraction
- `src/security/`: All security detectors updated with accurate line reporting
- `src/resource/`: All resource detectors updated with enhanced location tracking
- `src/analyzers/rust.rs`: Integration functions updated to provide source content to detectors

#### Integration Architecture
```
Files → Parse → AST + Source Content → Enhanced Detectors → Patterns with SourceLocation → Accurate DebtItems
                ↓                           ↓                        ↓
         UnifiedLocationExtractor → syn::Span extraction → Precise line numbers
```

## Dependencies

### Prerequisites
- **Spec 36**: Performance Detector Line Number Extraction
  - Provides SourceLocation structure and LocationExtractor foundation
  - Establishes patterns for syn::Span-based line extraction
  - Required for unified location architecture

### Affected Components
- `src/organization/`: All organization detector implementations
- `src/security/`: All security detector implementations  
- `src/resource/`: All resource detector implementations
- `src/analyzers/rust.rs`: Pattern analysis integration functions
- `src/core/`: DebtItem structure potentially enhanced with column information

### External Dependencies
- No new external dependencies required
- Uses existing syn, serde dependencies
- Leverages existing AST parsing and traversal infrastructure

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_god_object_accurate_line_numbers() {
        let source = r#"
use std::collections::HashMap;

struct UserManager {     // Line 4 ← Should be detected here
    users: HashMap<String, User>,
    sessions: HashMap<String, Session>,
    permissions: HashMap<String, Vec<Permission>>,
    audit_log: Vec<AuditEntry>,
    config: Config,
    cache: Cache,
    validator: Validator,
    notifier: Notifier,
}

impl UserManager {
    // 15+ methods making this a God object
    fn create_user(&self) { }
    fn update_user(&self) { }
    fn delete_user(&self) { }
    // ... many more methods
}
        "#;

        let file = syn::parse_str::<syn::File>(source).unwrap();
        let detector = GodObjectDetector::new(source);
        let patterns = detector.detect_anti_patterns(&file);

        assert!(!patterns.is_empty(), "Should detect God object pattern");
        
        if let OrganizationAntiPattern::GodObject { location, .. } = &patterns[0] {
            assert_eq!(location.line, 4, "Should detect God object on struct definition line 4");
            assert_eq!(location.confidence, LocationConfidence::Exact);
            assert!(location.column.is_some(), "Should include column information");
        } else {
            panic!("Expected God object pattern");
        }
    }

    #[test]
    fn test_input_validation_accurate_line_numbers() {
        let source = r#"
use std::io::Read;

fn process_user_input(request: UserRequest) -> Result<Response, Error> { // Line 4 ← Should be detected here
    // Missing input validation
    let data = request.data;
    process_data(data)
}

fn safe_function(internal_data: InternalData) -> Response {
    // No external input, should not be flagged
    process_internal(internal_data)
}
        "#;

        let file = syn::parse_str::<syn::File>(source).unwrap();
        let debt_items = detect_validation_gaps(&file, Path::new("test.rs"), source);

        assert!(!debt_items.is_empty(), "Should detect input validation gap");
        
        let validation_item = &debt_items[0];
        assert_eq!(validation_item.line, 4, "Should detect validation gap on function definition line 4");
        assert_ne!(validation_item.line, 0, "Should NOT use hardcoded line 0");
    }

    #[test]
    fn test_hardcoded_secret_accurate_line_numbers() {
        let source = r#"
const CONFIG: &str = "normal config";

fn connect_to_database() {
    let api_key = "REDACTED_TEST_API_KEY"; // Line 5 ← Should be detected here
    let connection = Database::connect(api_key);
}
        "#;

        let file = syn::parse_str::<syn::File>(source).unwrap();
        let debt_items = detect_hardcoded_secrets(&file, Path::new("test.rs"));

        assert!(!debt_items.is_empty(), "Should detect hardcoded secret");
        
        let secret_item = &debt_items[0];
        assert_eq!(secret_item.line, 5, "Should detect secret on line 5");
        assert_ne!(secret_item.line, 0, "Should NOT use hardcoded line 0");
    }

    #[test]
    fn test_missing_drop_accurate_line_numbers() {
        let source = r#"
use std::fs::File;

struct FileManager {    // Line 4 ← Should be detected here
    file_handle: File,
    temp_files: Vec<File>,
}

// Missing Drop implementation
        "#;

        let file = syn::parse_str::<syn::File>(source).unwrap();
        let detector = DropDetector::new(source);
        let issues = detector.detect_issues(&file, Path::new("test.rs"));

        if let Some(ResourceManagementIssue::MissingDrop { location, .. }) = issues.first() {
            assert_eq!(location.line, 4, "Should detect missing Drop on struct definition line 4");
            assert_eq!(location.confidence, LocationConfidence::Exact);
        } else {
            panic!("Expected missing Drop issue");
        }
    }
}
```

### Integration Tests

```rust
// tests/unified_line_extraction_integration.rs
use std::process::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_end_to_end_line_accuracy_all_detectors() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    
    // Create test file with known issues at specific lines
    fs::write(&test_file, r#"
use std::collections::HashMap;  // Line 2 - import (should NOT be flagged)

struct UserManager {            // Line 4 ← Organization: God object
    users: HashMap<String, User>,
    sessions: HashMap<String, Session>,
    permissions: HashMap<String, Vec<Permission>>,
    audit_log: Vec<AuditEntry>,
    config: Config,
    cache: Cache,
    validator: Validator,
    notifier: Notifier,
}

impl UserManager {
    fn process_user_input(&self, request: UserRequest) -> Response { // Line 16 ← Security: Input validation gap
        let api_key = "REDACTED_TEST_API_KEY";     // Line 17 ← Security: Hardcoded secret
        // Missing input validation
        process_data(request.data)
    }
}

struct ResourceHolder {         // Line 23 ← Resource: Missing Drop
    file_handle: std::fs::File,
    network_conn: TcpStream,
}
    "#).unwrap();

    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", test_file.to_str().unwrap(), "--comprehensive", "--detailed"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Verify that issues are reported on correct lines
    assert!(stdout.contains(":4"), "Should report God object on line 4");
    assert!(stdout.contains(":16"), "Should report input validation gap on line 16");
    assert!(stdout.contains(":17"), "Should report hardcoded secret on line 17");
    assert!(stdout.contains(":23"), "Should report missing Drop on line 23");
    
    // Verify that import lines are NOT flagged
    assert!(!stdout.contains(":2"), "Should NOT report issues on import line 2");
    
    // Verify that line 0 is not used
    assert!(!stdout.contains(":0"), "Should NOT report any issues on line 0");
}

#[test]
fn test_json_output_includes_accurate_line_numbers() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    
    fs::write(&test_file, r#"
struct GodObject {              // Line 2 ← Should be detected here
    field1: String, field2: String, field3: String, field4: String,
    field5: String, field6: String, field7: String, field8: String,
    field9: String, field10: String,
}

impl GodObject {
    fn method1(&self) { }
    fn method2(&self) { }
    fn method3(&self) { }
    fn method4(&self) { }
    fn method5(&self) { }
    fn method6(&self) { }
    fn method7(&self) { }
    fn method8(&self) { }
    fn method9(&self) { }
    fn method10(&self) { }
    fn method11(&self) { }
}
    "#).unwrap();

    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", test_file.to_str().unwrap(), "--format", "json"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    
    // Find organization debt items
    let debt_items = json["analysis"]["debt_items"].as_array().unwrap();
    let organization_items: Vec<_> = debt_items.iter()
        .filter(|item| item["debt_type"] == "CodeOrganization")
        .collect();
    
    assert!(!organization_items.is_empty(), "Should have organization debt items");
    
    // Verify line numbers are accurate
    for item in organization_items {
        let line = item["line"].as_u64().unwrap();
        assert_eq!(line, 2, "Organization issue should be on line 2, got line {}", line);
        assert_ne!(line, 0, "Should not use hardcoded line 0");
    }
}
```

### Performance Tests

```rust
#[test]
fn test_unified_line_extraction_performance() {
    use std::time::Instant;
    
    // Test with large file to ensure line extraction doesn't add significant overhead
    let large_source = generate_large_rust_file_with_mixed_patterns(1000); // 1000 functions, structs, patterns
    
    let start = Instant::now();
    let file = syn::parse_str::<syn::File>(&large_source).unwrap();
    let parse_time = start.elapsed();
    
    let start = Instant::now();
    // Run analysis with unified line extraction
    let organization_patterns = analyze_organization_patterns(&file, Path::new("large_test.rs"));
    let security_items = analyze_security_patterns(&file, Path::new("large_test.rs"));
    let resource_items = analyze_resource_patterns(&file, Path::new("large_test.rs"));
    let enhanced_time = start.elapsed();
    
    let start = Instant::now();
    // Run analysis without source content (fallback mode)
    let basic_patterns = analyze_organization_patterns_without_source(&file, Path::new("large_test.rs"));
    let basic_time = start.elapsed();
    
    // Unified line extraction should add <10% overhead
    let overhead_ratio = enhanced_time.as_nanos() as f64 / basic_time.as_nanos() as f64;
    assert!(overhead_ratio < 1.10, "Unified line extraction overhead too high: {:.2}%", (overhead_ratio - 1.0) * 100.0);
    
    // Verify patterns were detected with accurate locations
    assert!(!organization_patterns.is_empty(), "Should detect organization patterns");
    assert!(!security_items.is_empty(), "Should detect security issues");
    assert!(!resource_items.is_empty(), "Should detect resource issues");
    
    // Verify no line 0 issues
    for item in organization_patterns.iter().chain(security_items.iter()).chain(resource_items.iter()) {
        assert_ne!(item.line, 0, "Should not have line 0 issues");
    }
}
```

## Documentation Requirements

### Code Documentation
- Comprehensive rustdoc for UnifiedLocationExtractor and SourceLocation
- Examples of line number extraction for each detector type
- Migration guide for updating existing detector implementations
- Performance characteristics and accuracy limitations

### User Documentation
```markdown
## Accurate Source Locations

Debtmap now provides precise source locations for all detected issues across all analysis categories:

### Line Number Accuracy
- **Organization Issues**: God objects, magic values, and parameter issues reported at their actual definition sites
- **Security Issues**: Input validation gaps, hardcoded secrets, and SQL injection risks reported at their source locations  
- **Resource Issues**: Missing Drop implementations and resource leaks reported at their declaration sites
- **Performance Issues**: I/O, string, and loop issues reported at their operation sites

### Before/After Comparison
```bash
# Before: All issues at line 0 (unusable)
#1 SCORE: 8.3 [CRITICAL]
├─ ORGANIZATION: src/user.rs:0 organization_issue_at_line_0()  # ← Wrong: line 0
└─ WHY: God object detected with 15 methods

# After: Accurate source locations  
#1 SCORE: 8.3 [CRITICAL]
├─ ORGANIZATION: src/user.rs:42 UserManager                   # ← Correct: actual struct line
└─ WHY: God object detected with 15 methods at struct definition
```

### Enhanced Debugging Experience
All issues now include precise location information:
```
📊 ORGANIZATION ANALYSIS
├─ Issue: God object 'UserManager'
├─ Location: src/user_service.rs:42:1
├─ Confidence: Exact
└─ Context: Struct definition with 15 methods and 8 fields

🔒 SECURITY ANALYSIS  
├─ Issue: Hardcoded API key detected
├─ Location: src/config.rs:127:16
├─ Confidence: Exact
└─ Context: String literal within function scope

♻️ RESOURCE ANALYSIS
├─ Issue: Missing Drop implementation
├─ Location: src/file_manager.rs:23:1
├─ Confidence: Exact
└─ Context: Struct with File and TcpStream fields
```
```

### Architecture Documentation
Update ARCHITECTURE.md with unified line extraction architecture and cross-detector location consistency.

## Implementation Notes

### Phased Implementation
1. **Phase 1**: Unified SourceLocation infrastructure and UnifiedLocationExtractor utility
2. **Phase 2**: Organization detector line extraction (God object, Magic value, Parameter list)
3. **Phase 3**: Security detector line extraction (Input validation, Hardcoded secrets)
4. **Phase 4**: Resource detector line extraction (Missing Drop, Resource leaks)
5. **Phase 5**: Integration testing and performance validation

### Implementation Strategy
- **Clean Redesign**: Detector interfaces simplified to require source content from the start
- **Direct Implementation**: No gradual migration needed, immediate adoption of location tracking
- **Prototype Flexibility**: Breaking changes acceptable for rapid iteration and improvement
- **Test Validation**: Comprehensive tests ensure line number accuracy across all detectors

### Edge Cases to Consider
- **Macro-expanded code**: Use syn span information, fallback to original source locations
- **Missing source files**: Fail fast with clear error messages in prototype phase
- **Multi-line patterns**: Report primary line with optional range information
- **Generated code**: Clear indicators when location confidence is uncertain

## Expected Impact

After implementation:

1. **Restored Tool Usability**: Developers can immediately navigate to actual source of all detected issues
2. **Enhanced Developer Experience**: Precise locations enable direct code navigation and fixes across all issue types
3. **Increased Trust**: Accurate reporting builds confidence in tool reliability and recommendations
4. **Improved Workflow**: Integration with IDEs and editors becomes seamless with precise location information
5. **Complete Coverage**: All detector categories provide actionable location information

## Breaking Changes and Prototype Benefits

- **Breaking Changes**: Acceptable and beneficial - cleaner, simpler detector interfaces
- **API Redesign**: Detector constructors now require source content for immediate location tracking
- **Output Improvement**: Reports include precise location information from the start
- **Configuration**: No new configuration required - line extraction mandatory and automatic
- **Performance**: <10% overhead for comprehensive location accuracy across all detectors
- **Prototype Advantage**: Freedom to redesign interfaces optimally without legacy constraints

This specification addresses the systemic line number issue across all detector categories, providing a unified solution that makes debtmap a reliable and trustworthy development tool with accurate source location reporting for every detected issue.