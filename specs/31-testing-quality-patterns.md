---
number: 31
title: Testing Quality Patterns Detection
category: feature
priority: medium
status: draft
dependencies: []
created: 2025-08-16
---

# Specification 31: Testing Quality Patterns Detection

**Category**: feature
**Priority**: medium
**Status**: draft
**Dependencies**: []

## Context

Test quality significantly impacts codebase maintainability and reliability. Poor testing practices create technical debt that undermines confidence in code changes and slows development velocity. The current debtmap system lacks specific detection for testing-related technical debt patterns:

- **Tests Without Assertions** - Test functions that don't verify any behavior
- **Overly Complex Tests** - Tests with high complexity that are hard to understand and maintain
- **Missing Edge Case Coverage** - Tests that only cover happy paths
- **Test Data Duplication** - Repeated test setup code and data across tests
- **Flaky Test Patterns** - Tests with time dependencies, random values, or external dependencies
- **Insufficient Mocking** - Tests that don't properly isolate units under test

These testing anti-patterns represent technical debt that reduces the value of the test suite and creates maintenance overhead.

## Objective

Implement testing quality analysis that identifies test-specific anti-patterns not caught by existing test frameworks or language tooling:

1. **Test Structure Analysis**: Detect tests without assertions (not caught by any tools)
2. **Test Complexity Assessment**: Identify overly complex tests (unique metric)
3. **Test Duplication Analysis**: Find repeated test logic (not detected by frameworks)
4. **Language-Agnostic Patterns**: These issues exist across all languages and aren't caught by any existing tools
5. **Focus Areas**:
   - Tests that compile/run but don't actually test anything
   - Test code that's harder to understand than the code it tests
   - Test suites with poor maintainability characteristics

## Requirements

### Functional Requirements

1. **Test Structure Analysis**
   - Detect test functions without assertion statements
   - Identify tests with only setup code and no verification
   - Find tests that don't follow AAA (Arrange-Act-Assert) pattern
   - Detect tests with multiple responsibilities

2. **Test Complexity Assessment**
   - Apply complexity metrics specifically to test functions
   - Identify tests with excessive branching or loops
   - Detect tests with too many mock setups
   - Find tests that are longer than production code they test

3. **Coverage Gap Detection**
   - Identify missing error condition tests
   - Detect missing boundary value tests
   - Find untested public API methods
   - Identify missing integration test scenarios

4. **Test Data Management**
   - Detect duplicated test data across test files
   - Identify hardcoded test values that should be parameterized
   - Find test builders or factory patterns that could reduce duplication
   - Detect missing test data cleanup

5. **Flaky Test Detection**
   - Identify tests using Thread::sleep or timing-dependent logic
   - Detect tests with random value generation
   - Find tests accessing external services without mocking
   - Identify tests with file system dependencies

6. **Test Isolation Analysis**
   - Detect tests that modify global state
   - Identify tests that depend on execution order
   - Find tests sharing mutable test data
   - Detect missing dependency mocking

### Non-Functional Requirements

1. **Performance**
   - Testing analysis adds <15% overhead to total analysis time
   - Efficient test pattern recognition using AST analysis
   - Scalable to large test suites

2. **Accuracy**
   - >85% precision for test anti-pattern detection
   - >75% recall for significant test quality issues
   - Configurable thresholds for different test types

3. **Integration**
   - Works with existing test frameworks (built-in Rust test, criterion, etc.)
   - Integrates with test coverage analysis
   - Supports custom test attribute detection

## Acceptance Criteria

- [ ] **Test Without Assertions**: All test functions lacking proper assertions identified
- [ ] **Complex Tests**: Tests with excessive complexity flagged with simplification suggestions
- [ ] **Coverage Gaps**: Missing test scenarios identified with specific suggestions
- [ ] **Test Duplication**: Repeated test patterns detected with refactoring recommendations
- [ ] **Flaky Tests**: Non-deterministic test patterns identified with stabilization guidance
- [ ] **Test Isolation**: Tests with external dependencies or shared state flagged
- [ ] **Framework Support**: Works with standard Rust testing patterns and common frameworks
- [ ] **Actionable Feedback**: Each issue includes specific improvement recommendations

## Technical Details

### Implementation Approach

#### 1. Testing Quality Analysis Framework (`src/testing/`)

```rust
/// Testing quality anti-pattern detection framework
pub mod testing {
    use crate::core::ast::AstNode;
    use crate::core::{DebtItem, Priority};
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum TestingAntiPattern {
        TestWithoutAssertions {
            test_name: String,
            has_setup: bool,
            has_action: bool,
            suggested_assertions: Vec<String>,
        },
        OverlyComplexTest {
            test_name: String,
            complexity_score: u32,
            complexity_sources: Vec<ComplexitySource>,
            suggested_simplification: TestSimplification,
        },
        MissingEdgeCases {
            function_under_test: String,
            missing_scenarios: Vec<EdgeCaseScenario>,
            coverage_percentage: f64,
        },
        TestDataDuplication {
            duplicated_data: TestDataPattern,
            occurrence_count: usize,
            suggested_extraction: TestDataExtraction,
        },
        FlakyTestPattern {
            test_name: String,
            flakiness_type: FlakinessType,
            reliability_impact: ReliabilityImpact,
            stabilization_suggestion: String,
        },
        PoorTestIsolation {
            test_name: String,
            isolation_issue: IsolationIssue,
            affected_tests: Vec<String>,
            fixing_strategy: IsolationStrategy,
        },
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ComplexitySource {
        ExcessiveMocking,
        NestedConditionals,
        MultipleAssertions,
        LoopInTest,
        ExcessiveSetup,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum TestSimplification {
        ExtractHelper,
        SplitTest,
        ParameterizeTest,
        SimplifySetup,
        ReduceMocking,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum EdgeCaseScenario {
        NullInput,
        EmptyInput,
        BoundaryValues,
        ErrorConditions,
        ConcurrencyScenarios,
        InvalidInput,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum FlakinessType {
        TimingDependency,
        RandomValues,
        ExternalDependency,
        FilesystemDependency,
        NetworkDependency,
        ThreadingIssue,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum IsolationIssue {
        SharedMutableState,
        GlobalStateModification,
        ExecutionOrderDependency,
        ExternalServiceCall,
        FileSystemSideEffect,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ReliabilityImpact {
        Critical, // Test fails frequently
        High,     // Test fails occasionally  
        Medium,   // Test has timing issues
        Low,      // Minor reliability concerns
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub struct TestDataPattern {
        pub pattern_type: TestDataType,
        pub data_structure: String,
        pub usage_context: String,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum TestDataType {
        ObjectCreation,
        MockSetup,
        InputData,
        ExpectedResults,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum TestDataExtraction {
        TestFactory,
        FixtureFile,
        ParameterizedTest,
        TestBuilder,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum IsolationStrategy {
        MockDependencies,
        TestDoubles,
        StateReset,
        IsolatedEnvironment,
    }
    
    pub trait TestingDetector {
        fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<TestingAntiPattern>;
        fn detector_name(&self) -> &'static str;
        fn assess_test_quality_impact(&self, pattern: &TestingAntiPattern) -> TestQualityImpact;
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum TestQualityImpact {
        Critical, // Severely undermines test effectiveness
        High,     // Significantly reduces test value
        Medium,   // Moderately impacts test quality
        Low,      // Minor test quality issue
    }
}
```

#### 2. Test Without Assertions Detector (`src/testing/assertion_detector.rs`)

```rust
pub struct AssertionDetector {
    assertion_patterns: Vec<AssertionPattern>,
    test_attribute_patterns: Vec<String>,
}

impl TestingDetector for AssertionDetector {
    fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<TestingAntiPattern> {
        let mut patterns = Vec::new();
        let test_functions = self.find_test_functions(ast);
        
        for test_function in test_functions {
            let analysis = self.analyze_test_structure(&test_function);
            
            if !analysis.has_assertions {
                patterns.push(TestingAntiPattern::TestWithoutAssertions {
                    test_name: test_function.name.clone(),
                    has_setup: analysis.has_setup,
                    has_action: analysis.has_action,
                    suggested_assertions: self.suggest_assertions(&test_function, &analysis),
                });
            }
        }
        
        patterns
    }
}

impl AssertionDetector {
    fn find_test_functions(&self, ast: &AstNode) -> Vec<TestFunction> {
        let mut test_functions = Vec::new();
        
        ast.traverse_depth_first(|node| {
            if let AstNode::Function(function) = node {
                if self.is_test_function(function) {
                    test_functions.push(TestFunction {
                        name: function.name.clone(),
                        attributes: function.attributes.clone(),
                        body: function.body.clone(),
                        parameters: function.parameters.clone(),
                        return_type: function.return_type.clone(),
                    });
                }
            }
        });
        
        test_functions
    }
    
    fn is_test_function(&self, function: &FunctionNode) -> bool {
        // Check for #[test] attribute
        function.attributes.iter().any(|attr| {
            self.test_attribute_patterns.iter().any(|pattern| {
                attr.name.contains(pattern)
            })
        }) ||
        // Check for test naming conventions
        function.name.starts_with("test_") ||
        function.name.ends_with("_test")
    }
    
    fn analyze_test_structure(&self, test_function: &TestFunction) -> TestStructureAnalysis {
        let mut analysis = TestStructureAnalysis::default();
        
        // Analyze function body for different test phases
        test_function.body.traverse(|stmt| {
            match stmt {
                Statement::VariableAssignment(_) | 
                Statement::FunctionCall(call) if self.is_setup_call(call) => {
                    analysis.has_setup = true;
                }
                Statement::FunctionCall(call) if self.is_assertion_call(call) => {
                    analysis.has_assertions = true;
                    analysis.assertion_count += 1;
                }
                Statement::MethodCall(call) if self.is_action_call(call) => {
                    analysis.has_action = true;
                }
                Statement::MacroCall(call) if self.is_assertion_macro(call) => {
                    analysis.has_assertions = true;
                    analysis.assertion_count += 1;
                }
                _ => {}
            }
        });
        
        analysis
    }
    
    fn is_assertion_call(&self, call: &FunctionCall) -> bool {
        const ASSERTION_FUNCTIONS: &[&str] = &[
            "assert", "assert_eq", "assert_ne", "assert_matches",
            "debug_assert", "debug_assert_eq", "debug_assert_ne"
        ];
        
        ASSERTION_FUNCTIONS.contains(&call.function_name.as_str())
    }
    
    fn is_assertion_macro(&self, call: &MacroCall) -> bool {
        const ASSERTION_MACROS: &[&str] = &[
            "assert!", "assert_eq!", "assert_ne!", "assert_matches!",
            "panic!", "unreachable!", "todo!", "unimplemented!"
        ];
        
        ASSERTION_MACROS.contains(&call.macro_name.as_str())
    }
    
    fn suggest_assertions(&self, test_function: &TestFunction, analysis: &TestStructureAnalysis) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        if analysis.has_action && !analysis.has_assertions {
            // Look for variables that could be asserted
            let variables = self.extract_variables_from_test(test_function);
            
            for var in variables {
                match var.variable_type.as_str() {
                    "Result" => suggestions.push(format!("assert!({}. is_ok())", var.name)),
                    "Option" => suggestions.push(format!("assert!({}. is_some())", var.name)),
                    "bool" => suggestions.push(format!("assert!({})", var.name)),
                    "Vec" => suggestions.push(format!("assert!(!{}.is_empty())", var.name)),
                    _ => suggestions.push(format!("assert_eq!({}, expected_value)", var.name)),
                }
            }
        }
        
        if analysis.has_setup && !analysis.has_action {
            suggestions.push("Add action phase - call the method under test".to_string());
        }
        
        if !analysis.has_setup && !analysis.has_action {
            suggestions.push("Add complete test structure: setup -> action -> assert".to_string());
        }
        
        suggestions
    }
    
    fn extract_variables_from_test(&self, test_function: &TestFunction) -> Vec<TestVariable> {
        let mut variables = Vec::new();
        
        test_function.body.traverse(|stmt| {
            if let Statement::VariableAssignment(assignment) = stmt {
                variables.push(TestVariable {
                    name: assignment.variable_name.clone(),
                    variable_type: assignment.type_annotation.clone().unwrap_or_else(|| {
                        self.infer_type_from_assignment(&assignment.value)
                    }),
                    is_mutable: assignment.is_mutable,
                });
            }
        });
        
        variables
    }
}

#[derive(Debug, Default)]
struct TestStructureAnalysis {
    has_setup: bool,
    has_action: bool,
    has_assertions: bool,
    assertion_count: usize,
}

#[derive(Debug)]
struct TestVariable {
    name: String,
    variable_type: String,
    is_mutable: bool,
}
```

#### 3. Test Complexity Detector (`src/testing/complexity_detector.rs`)

```rust
pub struct TestComplexityDetector {
    max_test_complexity: u32,
    max_mock_setups: usize,
    max_test_length: usize,
}

impl TestingDetector for TestComplexityDetector {
    fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<TestingAntiPattern> {
        let mut patterns = Vec::new();
        let test_functions = self.find_test_functions(ast);
        
        for test_function in test_functions {
            let complexity_analysis = self.analyze_test_complexity(&test_function);
            
            if self.is_overly_complex(&complexity_analysis) {
                patterns.push(TestingAntiPattern::OverlyComplexTest {
                    test_name: test_function.name.clone(),
                    complexity_score: complexity_analysis.total_complexity,
                    complexity_sources: complexity_analysis.sources,
                    suggested_simplification: self.suggest_simplification(&complexity_analysis),
                });
            }
        }
        
        patterns
    }
}

impl TestComplexityDetector {
    fn analyze_test_complexity(&self, test_function: &TestFunction) -> TestComplexityAnalysis {
        let mut analysis = TestComplexityAnalysis::default();
        
        // Calculate cyclomatic complexity specific to tests
        analysis.cyclomatic_complexity = self.calculate_test_cyclomatic_complexity(&test_function.body);
        
        // Count mock setups
        analysis.mock_setup_count = self.count_mock_setups(&test_function.body);
        
        // Measure test length
        analysis.line_count = self.count_test_lines(&test_function.body);
        
        // Count assertion complexity
        analysis.assertion_complexity = self.calculate_assertion_complexity(&test_function.body);
        
        // Identify complexity sources
        analysis.sources = self.identify_complexity_sources(&test_function.body, &analysis);
        
        // Calculate total complexity score
        analysis.total_complexity = self.calculate_total_complexity_score(&analysis);
        
        analysis
    }
    
    fn count_mock_setups(&self, body: &FunctionBody) -> usize {
        let mut mock_count = 0;
        
        body.traverse(|stmt| {
            match stmt {
                Statement::FunctionCall(call) if self.is_mock_setup_call(call) => {
                    mock_count += 1;
                }
                Statement::MethodCall(call) if self.is_mock_method_call(call) => {
                    mock_count += 1;
                }
                _ => {}
            }
        });
        
        mock_count
    }
    
    fn is_mock_setup_call(&self, call: &FunctionCall) -> bool {
        const MOCK_FUNCTIONS: &[&str] = &[
            "mock", "when", "given", "expect", "stub", "fake",
            "with_return", "returns", "with_args", "times"
        ];
        
        MOCK_FUNCTIONS.iter().any(|mock_fn| {
            call.function_name.to_lowercase().contains(mock_fn)
        })
    }
    
    fn calculate_assertion_complexity(&self, body: &FunctionBody) -> u32 {
        let mut complexity = 0;
        
        body.traverse(|stmt| {
            if let Statement::MacroCall(call) = stmt {
                // Complex assertions add to test complexity
                match call.macro_name.as_str() {
                    "assert_matches!" => complexity += 2,
                    "assert!" if self.has_complex_expression(&call.arguments) => complexity += 2,
                    "assert_eq!" | "assert_ne!" => complexity += 1,
                    _ => {}
                }
            }
        });
        
        complexity
    }
    
    fn has_complex_expression(&self, arguments: &[MacroArgument]) -> bool {
        // Check if assertion contains complex boolean logic
        arguments.iter().any(|arg| {
            arg.contains_pattern("&&") || 
            arg.contains_pattern("||") ||
            arg.contains_pattern("match") ||
            arg.contains_pattern("if")
        })
    }
    
    fn suggest_simplification(&self, analysis: &TestComplexityAnalysis) -> TestSimplification {
        if analysis.mock_setup_count > self.max_mock_setups {
            TestSimplification::ReduceMocking
        } else if analysis.line_count > self.max_test_length {
            if analysis.has_multiple_concerns() {
                TestSimplification::SplitTest
            } else {
                TestSimplification::ExtractHelper
            }
        } else if analysis.cyclomatic_complexity > 5 {
            TestSimplification::ParameterizeTest
        } else {
            TestSimplification::SimplifySetup
        }
    }
    
    fn calculate_total_complexity_score(&self, analysis: &TestComplexityAnalysis) -> u32 {
        analysis.cyclomatic_complexity +
        (analysis.mock_setup_count as u32 * 2) +
        analysis.assertion_complexity +
        (analysis.line_count as u32 / 10) // Penalty for long tests
    }
}

#[derive(Debug, Default)]
struct TestComplexityAnalysis {
    cyclomatic_complexity: u32,
    mock_setup_count: usize,
    line_count: usize,
    assertion_complexity: u32,
    total_complexity: u32,
    sources: Vec<ComplexitySource>,
}

impl TestComplexityAnalysis {
    fn has_multiple_concerns(&self) -> bool {
        // Heuristic: if test has many different types of operations, it might test multiple concerns
        self.mock_setup_count > 3 && self.assertion_complexity > 3
    }
}
```

#### 4. Flaky Test Detector (`src/testing/flaky_detector.rs`)

```rust
pub struct FlakyTestDetector {
    timing_patterns: Vec<TimingPattern>,
    external_dependency_patterns: Vec<String>,
}

impl TestingDetector for FlakyTestDetector {
    fn detect_anti_patterns(&self, ast: &AstNode) -> Vec<TestingAntiPattern> {
        let mut patterns = Vec::new();
        let test_functions = self.find_test_functions(ast);
        
        for test_function in test_functions {
            let flakiness_analysis = self.analyze_flakiness(&test_function);
            
            if !flakiness_analysis.flakiness_indicators.is_empty() {
                for indicator in flakiness_analysis.flakiness_indicators {
                    patterns.push(TestingAntiPattern::FlakyTestPattern {
                        test_name: test_function.name.clone(),
                        flakiness_type: indicator.flakiness_type,
                        reliability_impact: indicator.impact,
                        stabilization_suggestion: indicator.suggestion,
                    });
                }
            }
        }
        
        patterns
    }
}

impl FlakyTestDetector {
    fn analyze_flakiness(&self, test_function: &TestFunction) -> FlakinessAnalysis {
        let mut analysis = FlakinessAnalysis::default();
        
        test_function.body.traverse(|stmt| {
            self.check_timing_dependencies(stmt, &mut analysis);
            self.check_random_values(stmt, &mut analysis);
            self.check_external_dependencies(stmt, &mut analysis);
            self.check_filesystem_dependencies(stmt, &mut analysis);
            self.check_threading_issues(stmt, &mut analysis);
        });
        
        analysis
    }
    
    fn check_timing_dependencies(&self, stmt: &Statement, analysis: &mut FlakinessAnalysis) {
        match stmt {
            Statement::FunctionCall(call) => {
                if self.is_timing_function(call) {
                    analysis.flakiness_indicators.push(FlakinessIndicator {
                        flakiness_type: FlakinessType::TimingDependency,
                        impact: ReliabilityImpact::High,
                        suggestion: "Replace sleep/timing dependencies with deterministic waits or mocks".to_string(),
                        location: call.location.clone(),
                    });
                }
            }
            Statement::MacroCall(call) => {
                if call.macro_name == "thread::sleep" {
                    analysis.flakiness_indicators.push(FlakinessIndicator {
                        flakiness_type: FlakinessType::TimingDependency,
                        impact: ReliabilityImpact::Critical,
                        suggestion: "Remove thread::sleep and use deterministic synchronization".to_string(),
                        location: call.location.clone(),
                    });
                }
            }
            _ => {}
        }
    }
    
    fn check_random_values(&self, stmt: &Statement, analysis: &mut FlakinessAnalysis) {
        if let Statement::FunctionCall(call) = stmt {
            if self.is_random_function(call) {
                analysis.flakiness_indicators.push(FlakinessIndicator {
                    flakiness_type: FlakinessType::RandomValues,
                    impact: ReliabilityImpact::Medium,
                    suggestion: "Use deterministic test data instead of random values".to_string(),
                    location: call.location.clone(),
                });
            }
        }
    }
    
    fn check_external_dependencies(&self, stmt: &Statement, analysis: &mut FlakinessAnalysis) {
        if let Statement::FunctionCall(call) = stmt {
            if self.is_external_service_call(call) {
                analysis.flakiness_indicators.push(FlakinessIndicator {
                    flakiness_type: FlakinessType::ExternalDependency,
                    impact: ReliabilityImpact::Critical,
                    suggestion: "Mock external service calls for unit tests".to_string(),
                    location: call.location.clone(),
                });
            }
        }
    }
    
    fn is_timing_function(&self, call: &FunctionCall) -> bool {
        const TIMING_FUNCTIONS: &[&str] = &[
            "sleep", "delay", "timeout", "wait", "pause",
            "now", "elapsed", "duration", "instant"
        ];
        
        TIMING_FUNCTIONS.iter().any(|timing_fn| {
            call.function_name.to_lowercase().contains(timing_fn)
        })
    }
    
    fn is_random_function(&self, call: &FunctionCall) -> bool {
        const RANDOM_FUNCTIONS: &[&str] = &[
            "rand", "random", "rng", "thread_rng", "gen",
            "choose", "shuffle", "sample"
        ];
        
        RANDOM_FUNCTIONS.iter().any(|random_fn| {
            call.function_name.to_lowercase().contains(random_fn)
        })
    }
    
    fn is_external_service_call(&self, call: &FunctionCall) -> bool {
        const EXTERNAL_PATTERNS: &[&str] = &[
            "http", "request", "client", "api", "service",
            "database", "db", "sql", "query", "connection"
        ];
        
        EXTERNAL_PATTERNS.iter().any(|pattern| {
            call.function_name.to_lowercase().contains(pattern) ||
            call.module_path.to_lowercase().contains(pattern)
        })
    }
}

#[derive(Debug, Default)]
struct FlakinessAnalysis {
    flakiness_indicators: Vec<FlakinessIndicator>,
}

#[derive(Debug)]
struct FlakinessIndicator {
    flakiness_type: FlakinessType,
    impact: ReliabilityImpact,
    suggestion: String,
    location: SourceLocation,
}
```

#### 5. Integration with Main Analysis Pipeline

```rust
// In src/analyzers/rust.rs
use crate::testing::{
    TestingDetector, AssertionDetector, TestComplexityDetector,
    FlakyTestDetector, TestDataDuplicationDetector, TestIsolationDetector
};

fn analyze_testing_patterns(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    let detectors: Vec<Box<dyn TestingDetector>> = vec![
        Box::new(AssertionDetector::new()),
        Box::new(TestComplexityDetector::new()),
        Box::new(FlakyTestDetector::new()),
        Box::new(TestDataDuplicationDetector::new()),
        Box::new(TestIsolationDetector::new()),
    ];
    
    let ast_node = convert_syn_to_ast_node(file);
    let mut testing_items = Vec::new();
    
    for detector in detectors {
        let anti_patterns = detector.detect_anti_patterns(&ast_node);
        
        for pattern in anti_patterns {
            let impact = detector.assess_test_quality_impact(&pattern);
            let debt_item = convert_testing_pattern_to_debt_item(pattern, impact, path);
            testing_items.push(debt_item);
        }
    }
    
    testing_items
}

fn convert_testing_pattern_to_debt_item(
    pattern: TestingAntiPattern,
    impact: TestQualityImpact,
    path: &Path
) -> DebtItem {
    let (priority, message, context) = match pattern {
        TestingAntiPattern::TestWithoutAssertions { test_name, suggested_assertions, .. } => {
            (
                Priority::High,
                format!("Test '{}' has no assertions", test_name),
                Some(format!("Add assertions: {}", suggested_assertions.join(", ")))
            )
        }
        TestingAntiPattern::OverlyComplexTest { test_name, complexity_score, suggested_simplification, .. } => {
            (
                Priority::Medium,
                format!("Test '{}' is overly complex (score: {})", test_name, complexity_score),
                Some(format!("Consider: {:?}", suggested_simplification))
            )
        }
        TestingAntiPattern::FlakyTestPattern { test_name, flakiness_type, stabilization_suggestion, .. } => {
            (
                Priority::High,
                format!("Test '{}' has flaky pattern: {:?}", test_name, flakiness_type),
                Some(stabilization_suggestion)
            )
        }
        TestingAntiPattern::TestDataDuplication { duplicated_data, occurrence_count, suggested_extraction, .. } => {
            (
                Priority::Low,
                format!("Test data duplication in {} tests", occurrence_count),
                Some(format!("Extract using: {:?}", suggested_extraction))
            )
        }
        TestingAntiPattern::PoorTestIsolation { test_name, isolation_issue, fixing_strategy, .. } => {
            (
                Priority::Medium,
                format!("Test '{}' has isolation issue: {:?}", test_name, isolation_issue),
                Some(format!("Fix with: {:?}", fixing_strategy))
            )
        }
        TestingAntiPattern::MissingEdgeCases { function_under_test, missing_scenarios, .. } => {
            (
                Priority::Medium,
                format!("Function '{}' missing edge case tests", function_under_test),
                Some(format!("Add tests for: {:?}", missing_scenarios))
            )
        }
    };
    
    DebtItem {
        id: format!("testing-{}-{}", path.display(), get_line_from_pattern(&pattern)),
        debt_type: DebtType::TestQuality, // New debt type
        priority,
        file: path.to_path_buf(),
        line: get_line_from_pattern(&pattern),
        message,
        context,
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_assertion_detection() {
        let source = r#"
            #[test]
            fn test_without_assertions() {
                let user = User::new("test");
                let result = user.validate();
                // Missing assertion!
            }
            
            #[test] 
            fn test_with_assertions() {
                let user = User::new("test");
                let result = user.validate();
                assert!(result.is_ok());
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = AssertionDetector::new();
        let patterns = detector.detect_anti_patterns(&ast);
        
        assert_eq!(patterns.len(), 1);
        if let TestingAntiPattern::TestWithoutAssertions { test_name, .. } = &patterns[0] {
            assert_eq!(test_name, "test_without_assertions");
        } else {
            panic!("Expected test without assertions pattern");
        }
    }
    
    #[test]
    fn test_flaky_test_detection() {
        let source = r#"
            #[test]
            fn flaky_timing_test() {
                let start = std::time::Instant::now();
                std::thread::sleep(std::time::Duration::from_millis(100));
                let duration = start.elapsed();
                assert!(duration >= std::time::Duration::from_millis(90));
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = FlakyTestDetector::new();
        let patterns = detector.detect_anti_patterns(&ast);
        
        assert!(!patterns.is_empty());
        let timing_pattern = patterns.iter().find(|p| {
            matches!(p, TestingAntiPattern::FlakyTestPattern { 
                flakiness_type: FlakinessType::TimingDependency, 
                .. 
            })
        });
        assert!(timing_pattern.is_some());
    }
    
    #[test]
    fn test_complex_test_detection() {
        let source = r#"
            #[test]
            fn overly_complex_test() {
                // Setup with many mocks
                let mut mock1 = MockService::new();
                let mut mock2 = MockDatabase::new();
                let mut mock3 = MockCache::new();
                
                mock1.expect_call().times(1).returning(|| Ok(()));
                mock2.expect_query().with(eq("test")).returning(|| Ok(vec![]));
                mock3.expect_get().returning(|| None);
                
                // Complex logic in test
                for i in 0..10 {
                    if i % 2 == 0 {
                        let result = service.process(i);
                        if let Some(data) = result {
                            assert_eq!(data.len(), i);
                        } else {
                            panic!("Unexpected None");
                        }
                    }
                }
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = TestComplexityDetector::new();
        let patterns = detector.detect_anti_patterns(&ast);
        
        assert!(!patterns.is_empty());
        if let TestingAntiPattern::OverlyComplexTest { complexity_score, .. } = &patterns[0] {
            assert!(complexity_score > &10);
        } else {
            panic!("Expected overly complex test pattern");
        }
    }
}
```

## Configuration

```toml
[testing]
enabled = true
detectors = ["assertions", "complexity", "flakiness", "duplication", "isolation"]

[testing.assertions]
require_assertions = true
suggest_assertion_types = true
aaa_pattern_enforcement = true

[testing.complexity]
max_test_complexity = 10
max_mock_setups = 5
max_test_length = 50

[testing.flakiness]
detect_timing_dependencies = true
detect_random_values = true
detect_external_dependencies = true

[testing.duplication]
min_duplication_threshold = 3
suggest_test_factories = true
suggest_parameterized_tests = true

[testing.isolation]
detect_shared_state = true
detect_execution_order_dependencies = true
suggest_mocking_strategies = true
```

## Expected Impact

After implementation:

1. **Test Effectiveness**: Higher quality tests that actually verify behavior
2. **Test Reliability**: Reduction in flaky tests that cause CI/CD issues
3. **Test Maintainability**: Simpler, more focused tests that are easier to understand
4. **Test Coverage**: Better edge case coverage through systematic gap detection
5. **Development Velocity**: More reliable test suite enables faster development cycles

This testing quality analysis complements existing technical debt detection by focusing specifically on test-related issues that impact the reliability and maintainability of the test suite, which is crucial for sustainable software development.