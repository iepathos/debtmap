---
number: 42
title: Smart Pattern Matching for Performance Detection
category: optimization
priority: high
status: draft
dependencies: [41, 35, 28]
created: 2025-08-17
---

# Specification 42: Smart Pattern Matching for Performance Detection

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [41 - Test Performance as Tech Debt, 35 - Debt Pattern Unified Scoring Integration, 28 - Security Patterns Detection]

## Context

The current performance detection system in debtmap (IOPerformanceDetector, NestedLoopDetector, etc.) produces false positives that undermine user confidence in the tool's recommendations. Specifically, the blocking I/O detection flags legitimate patterns as performance issues without understanding their semantic context:

### Current False Positive Issues

1. **Test File I/O Operations**: The blocking I/O detector correctly identifies `std::fs::write()` calls in test loops but fails to distinguish between intentional test fixtures and production performance issues
2. **Context-Blind Detection**: Pattern matching relies solely on AST structure without considering:
   - Function purpose and intent
   - Code location and module type  
   - Architectural patterns (setup/teardown, mocking, fixtures)
   - Developer intent signals (comments, naming conventions)
3. **One-Size-Fits-All Severity**: All blocking I/O patterns receive the same priority regardless of actual performance impact or business criticality

### Example False Positive
```rust
// tests/core_cache_tests.rs - Line 45
for i in 0..5 {
    let test_file = temp_dir.path().join(format!("test_{}.rs", i));
    std::fs::write(&test_file, "test content").unwrap(); // Flagged as blocking I/O
}
```

This is correctly identified as blocking I/O in a loop, but the context shows it's test fixture setup, not a production performance issue.

## Objective

Implement intelligent pattern matching that combines AST-based detection with semantic analysis to distinguish between legitimate performance issues and acceptable patterns. This will reduce false positives by 70%+ while maintaining sensitivity to real performance problems.

## Requirements

### Functional Requirements

1. **Context-Aware Pattern Detection**
   - Semantic function classification (setup/teardown, business logic, utilities)
   - Module type detection (tests, benchmarks, examples, documentation)
   - Intent recognition through naming patterns and documentation
   - Architectural pattern recognition (builder, factory, visitor patterns)

2. **Smart Severity Adjustment**  
   - Dynamic priority scaling based on detected context
   - Business impact assessment using call graph analysis
   - Performance criticality scoring based on usage patterns
   - Configurable context-specific thresholds

3. **Enhanced Pattern Matching**
   - Multi-pattern correlation (e.g., I/O + error handling + testing context)
   - Temporal pattern analysis (setup → operation → cleanup sequences)
   - Cross-module pattern detection for distributed architectures
   - Framework-aware detection (test frameworks, web frameworks, CLI patterns)

4. **Explainable Recommendations**
   - Clear reasoning for why patterns are flagged or dismissed
   - Context-specific guidance that acknowledges legitimate use cases
   - Differentiated recommendations for different pattern contexts
   - Confidence scoring with uncertainty acknowledgment

### Non-Functional Requirements

1. **Performance**
   - Pattern analysis adds <15% overhead to existing detection
   - Efficient context caching to avoid redundant analysis
   - Incremental pattern matching for large codebases
   - Memory-efficient semantic analysis pipeline

2. **Accuracy**
   - >85% precision in context classification (low false positives)
   - >90% recall for genuine performance issues (high sensitivity)
   - Configurable confidence thresholds for pattern matching
   - Graceful degradation when context is ambiguous

3. **Extensibility**
   - Plugin architecture for domain-specific pattern matchers
   - Configurable pattern definitions via configuration files
   - Language-agnostic pattern matching framework
   - Framework-specific pattern recognition modules

4. **Maintainability**
   - Clear separation between detection, analysis, and recommendation phases
   - Comprehensive test coverage for pattern matching logic
   - Well-documented heuristics and their rationale
   - Modular design following functional programming principles

## Acceptance Criteria

- [ ] **Context Classification**: Test files, benchmarks, and examples are correctly identified and receive reduced severity
- [ ] **Intent Recognition**: Functions with setup/teardown naming patterns are properly classified
- [ ] **Framework Detection**: Common test framework patterns (fixture setup, mocking) are recognized
- [ ] **Business Impact Scoring**: Performance issues in critical paths receive higher priority than utility functions
- [ ] **Pattern Correlation**: Related patterns (I/O + error handling + testing) are analyzed together for better accuracy
- [ ] **Explainable Output**: Each recommendation includes clear reasoning for classification decisions
- [ ] **False Positive Reduction**: 70%+ reduction in false positives while maintaining detection of real issues
- [ ] **Configuration Support**: All matching thresholds and context weights are configurable
- [ ] **Performance Impact**: Analysis completes within 15% of baseline detection time
- [ ] **Integration Testing**: End-to-end validation with real-world codebases shows improved accuracy

## Technical Details

### Implementation Approach

#### 1. Context Analysis Engine (`src/performance/context/`)

```rust
/// Core context analysis framework
pub mod context {
    use crate::core::ast::AstNode;
    use crate::performance::PerformanceAntiPattern;
    
    #[derive(Debug, Clone, PartialEq)]
    pub struct PatternContext {
        pub module_type: ModuleType,
        pub function_intent: FunctionIntent,
        pub architectural_pattern: Option<ArchitecturalPattern>,
        pub business_criticality: BusinessCriticality,
        pub performance_sensitivity: PerformanceSensitivity,
        pub confidence: f64,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ModuleType {
        Production,
        Test,
        Benchmark,
        Example,
        Documentation,
        Utility,
        Infrastructure,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum FunctionIntent {
        BusinessLogic,
        Setup,
        Teardown,
        Validation,
        DataTransformation,
        IOWrapper,
        ErrorHandling,
        Configuration,
        Unknown,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ArchitecturalPattern {
        TestFixture,
        Builder,
        Factory,
        Repository,
        ServiceLayer,
        EventHandler,
        Middleware,
        DataAccess,
    }
    
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum BusinessCriticality {
        Critical,    // Core business logic, hot paths
        Important,   // Supporting business operations
        Utility,     // Helper functions, utilities
        Infrastructure, // Framework, configuration
        Development, // Tests, examples, debugging
    }
    
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum PerformanceSensitivity {
        High,        // Real-time, hot paths, user-facing
        Medium,      // Batch processing, background tasks
        Low,         // Setup, configuration, one-time operations
        Irrelevant,  // Tests, examples, debugging
    }
    
    pub trait ContextAnalyzer {
        fn analyze_context(&self, ast: &AstNode, file_path: &Path) -> PatternContext;
        fn analyze_function_context(&self, function: &FunctionNode, module_context: &PatternContext) -> PatternContext;
    }
}
```

#### 2. Module Type Detection (`src/performance/context/module_classifier.rs`)

```rust
pub struct ModuleClassifier {
    test_patterns: Vec<PathPattern>,
    benchmark_patterns: Vec<PathPattern>,
    example_patterns: Vec<PathPattern>,
    doc_patterns: Vec<PathPattern>,
}

impl ModuleClassifier {
    pub fn classify_module(&self, file_path: &Path) -> ModuleType {
        let path_str = file_path.to_string_lossy().to_lowercase();
        
        // Check explicit test directories and files
        if self.is_test_module(&path_str) {
            return ModuleType::Test;
        }
        
        // Check benchmark patterns
        if self.is_benchmark_module(&path_str) {
            return ModuleType::Benchmark;
        }
        
        // Check example patterns
        if self.is_example_module(&path_str) {
            return ModuleType::Example;
        }
        
        // Check documentation patterns
        if self.is_documentation_module(&path_str) {
            return ModuleType::Documentation;
        }
        
        // Analyze content for utility vs production
        if self.is_utility_module(&path_str) {
            ModuleType::Utility
        } else {
            ModuleType::Production
        }
    }
    
    fn is_test_module(&self, path: &str) -> bool {
        // Standard test patterns
        path.starts_with("tests/") ||
        path.contains("/tests/") ||
        path.ends_with("_test.rs") ||
        path.ends_with("_tests.rs") ||
        path.ends_with("/test.rs") ||
        path.contains("test_") ||
        // Integration test patterns
        path.contains("integration") && path.contains("test") ||
        // Framework-specific patterns
        path.contains("spec/") ||
        path.contains("__tests__/")
    }
    
    fn is_benchmark_module(&self, path: &str) -> bool {
        path.starts_with("benches/") ||
        path.contains("/benches/") ||
        path.contains("benchmark") ||
        path.contains("_bench.rs") ||
        path.contains("perf_")
    }
}
```

#### 3. Function Intent Analysis (`src/performance/context/intent_classifier.rs`)

```rust
pub struct IntentClassifier {
    setup_patterns: Vec<String>,
    teardown_patterns: Vec<String>,
    business_logic_indicators: Vec<String>,
    io_wrapper_patterns: Vec<String>,
}

impl IntentClassifier {
    pub fn classify_function_intent(
        &self, 
        function: &FunctionNode,
        call_graph: &CallGraph
    ) -> FunctionIntent {
        let function_name = function.name.to_lowercase();
        
        // Check setup/teardown patterns
        if self.is_setup_function(&function_name) {
            return FunctionIntent::Setup;
        }
        
        if self.is_teardown_function(&function_name) {
            return FunctionIntent::Teardown;
        }
        
        // Analyze function body and call patterns
        let body_analysis = self.analyze_function_body(function);
        
        // Check for validation patterns
        if self.is_validation_function(&function_name, &body_analysis) {
            return FunctionIntent::Validation;
        }
        
        // Check for I/O wrapper patterns
        if self.is_io_wrapper(function, call_graph) {
            return FunctionIntent::IOWrapper;
        }
        
        // Check for data transformation patterns
        if self.is_data_transformation(function, &body_analysis) {
            return FunctionIntent::DataTransformation;
        }
        
        // Default to business logic for production code
        FunctionIntent::BusinessLogic
    }
    
    fn is_setup_function(&self, name: &str) -> bool {
        let setup_keywords = [
            "setup", "setUp", "before", "init", "initialize", "create",
            "prepare", "arrange", "given", "fixture", "mock", 
            "stub", "build", "construct", "configure"
        ];
        
        setup_keywords.iter().any(|keyword| name.contains(keyword)) ||
        name.starts_with("test_") && (
            name.contains("setup") || 
            name.contains("prepare") ||
            name.contains("create")
        )
    }
    
    fn is_teardown_function(&self, name: &str) -> bool {
        let teardown_keywords = [
            "teardown", "tearDown", "cleanup", "clean", "after",
            "destroy", "dispose", "finalize", "reset", "clear",
            "remove", "delete", "drop"
        ];
        
        teardown_keywords.iter().any(|keyword| name.contains(keyword))
    }
    
    fn is_io_wrapper(&self, function: &FunctionNode, call_graph: &CallGraph) -> bool {
        // Function primarily delegates to I/O operations
        let io_call_ratio = self.calculate_io_call_ratio(function, call_graph);
        let business_logic_complexity = self.estimate_business_logic_complexity(function);
        
        io_call_ratio > 0.7 && business_logic_complexity < 3.0
    }
}
```

#### 4. Smart Performance Detector (`src/performance/smart_detector.rs`)

```rust
pub struct SmartPerformanceDetector {
    base_detectors: Vec<Box<dyn PerformanceDetector>>,
    context_analyzer: ContextAnalyzer,
    pattern_matcher: PatternMatcher,
    severity_adjuster: SeverityAdjuster,
}

impl SmartPerformanceDetector {
    pub fn detect_with_context(
        &self, 
        file: &syn::File, 
        path: &Path,
        call_graph: &CallGraph
    ) -> Vec<SmartPerformanceIssue> {
        // Step 1: Run base detection
        let mut raw_patterns = Vec::new();
        for detector in &self.base_detectors {
            raw_patterns.extend(detector.detect_anti_patterns(file, path));
        }
        
        if raw_patterns.is_empty() {
            return Vec::new();
        }
        
        // Step 2: Analyze context
        let module_context = self.context_analyzer.analyze_module_context(file, path);
        
        // Step 3: Analyze each pattern with context
        let mut smart_issues = Vec::new();
        for pattern in raw_patterns {
            let function_context = self.analyze_pattern_context(&pattern, file, &module_context, call_graph);
            let confidence = self.calculate_pattern_confidence(&pattern, &function_context);
            
            // Step 4: Apply smart filtering
            if self.should_report_pattern(&pattern, &function_context, confidence) {
                let adjusted_severity = self.severity_adjuster.adjust_severity(
                    &pattern, 
                    &function_context, 
                    confidence
                );
                
                smart_issues.push(SmartPerformanceIssue {
                    original_pattern: pattern,
                    context: function_context,
                    adjusted_severity,
                    confidence,
                    reasoning: self.generate_reasoning(&pattern, &function_context),
                    recommendation: self.generate_contextual_recommendation(&pattern, &function_context),
                });
            }
        }
        
        smart_issues
    }
    
    fn should_report_pattern(
        &self,
        pattern: &PerformanceAntiPattern,
        context: &PatternContext,
        confidence: f64
    ) -> bool {
        // Use configurable thresholds
        let config = crate::config::get_smart_performance_config();
        
        // Always report if confidence is very high
        if confidence >= config.high_confidence_threshold {
            return true;
        }
        
        // Filter based on context
        match (context.module_type, context.function_intent, context.performance_sensitivity) {
            // Never report test fixture setup/teardown
            (ModuleType::Test, FunctionIntent::Setup | FunctionIntent::Teardown, _) => false,
            
            // Report test business logic with reduced severity
            (ModuleType::Test, FunctionIntent::BusinessLogic, _) => {
                confidence >= config.test_confidence_threshold
            }
            
            // Report utility functions only if high confidence
            (_, _, PerformanceSensitivity::Irrelevant) => {
                confidence >= config.utility_confidence_threshold
            }
            
            // Report production code with normal thresholds
            (ModuleType::Production, _, PerformanceSensitivity::High | PerformanceSensitivity::Medium) => {
                confidence >= config.production_confidence_threshold
            }
            
            // Default: report if above base threshold
            _ => confidence >= config.base_confidence_threshold,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SmartPerformanceIssue {
    pub original_pattern: PerformanceAntiPattern,
    pub context: PatternContext,
    pub adjusted_severity: Priority,
    pub confidence: f64,
    pub reasoning: String,
    pub recommendation: String,
}
```

#### 5. Severity Adjustment Engine (`src/performance/context/severity_adjuster.rs`)

```rust
pub struct SeverityAdjuster {
    context_weights: ContextWeights,
}

#[derive(Debug, Clone)]
pub struct ContextWeights {
    pub module_type_weight: f64,
    pub function_intent_weight: f64,
    pub business_criticality_weight: f64,
    pub performance_sensitivity_weight: f64,
    pub architectural_pattern_weight: f64,
}

impl SeverityAdjuster {
    pub fn adjust_severity(
        &self,
        pattern: &PerformanceAntiPattern,
        context: &PatternContext,
        confidence: f64
    ) -> Priority {
        let base_severity = self.get_base_severity(pattern);
        let context_adjustment = self.calculate_context_adjustment(context);
        let confidence_adjustment = self.calculate_confidence_adjustment(confidence);
        
        let adjusted_score = base_severity as f64 * context_adjustment * confidence_adjustment;
        
        self.score_to_priority(adjusted_score)
    }
    
    fn calculate_context_adjustment(&self, context: &PatternContext) -> f64 {
        let mut adjustment = 1.0;
        
        // Module type adjustment
        adjustment *= match context.module_type {
            ModuleType::Production => 1.0,
            ModuleType::Test => 0.3,         // Significant reduction for tests
            ModuleType::Benchmark => 0.1,    // Benchmarks are expected to stress-test
            ModuleType::Example => 0.2,      // Examples are for demonstration
            ModuleType::Documentation => 0.1, // Doc tests are simple
            ModuleType::Utility => 0.7,      // Utility functions matter but less critical
            ModuleType::Infrastructure => 0.5, // Infrastructure should be efficient but not critical
        };
        
        // Function intent adjustment
        adjustment *= match context.function_intent {
            FunctionIntent::BusinessLogic => 1.0,
            FunctionIntent::Setup => 0.2,          // Setup is typically one-time
            FunctionIntent::Teardown => 0.1,       // Teardown even less critical
            FunctionIntent::Validation => 0.8,     // Validation should be efficient
            FunctionIntent::DataTransformation => 0.9, // Data transformation is important
            FunctionIntent::IOWrapper => 0.4,      // I/O wrappers expected to do I/O
            FunctionIntent::ErrorHandling => 0.6,  // Error handling should be fast but not critical
            FunctionIntent::Configuration => 0.3,  // Configuration is typically one-time
            FunctionIntent::Unknown => 0.8,        // Be conservative when uncertain
        };
        
        // Performance sensitivity adjustment
        adjustment *= match context.performance_sensitivity {
            PerformanceSensitivity::High => 1.5,      // Boost for hot paths
            PerformanceSensitivity::Medium => 1.0,
            PerformanceSensitivity::Low => 0.5,
            PerformanceSensitivity::Irrelevant => 0.1,
        };
        
        // Business criticality adjustment
        adjustment *= match context.business_criticality {
            BusinessCriticality::Critical => 1.3,     // Boost for critical business logic
            BusinessCriticality::Important => 1.0,
            BusinessCriticality::Utility => 0.7,
            BusinessCriticality::Infrastructure => 0.6,
            BusinessCriticality::Development => 0.2,  // Development code is less critical
        };
        
        adjustment.max(0.01) // Never reduce to zero
    }
}
```

#### 6. Pattern Correlation Engine (`src/performance/context/pattern_correlator.rs`)

```rust
pub struct PatternCorrelator {
    correlation_rules: Vec<CorrelationRule>,
}

#[derive(Debug, Clone)]
pub struct CorrelationRule {
    pub patterns: Vec<PatternType>,
    pub context_indicators: Vec<ContextIndicator>,
    pub confidence_adjustment: f64,
    pub severity_adjustment: f64,
    pub explanation: String,
}

impl PatternCorrelator {
    pub fn correlate_patterns(
        &self,
        patterns: &[PerformanceAntiPattern],
        contexts: &[PatternContext]
    ) -> Vec<PatternCorrelation> {
        let mut correlations = Vec::new();
        
        // Look for test fixture patterns
        if self.has_test_fixture_pattern(patterns, contexts) {
            correlations.push(PatternCorrelation {
                pattern_group: patterns.to_vec(),
                correlation_type: CorrelationType::TestFixture,
                confidence_boost: 0.9,
                severity_reduction: 0.2,
                explanation: "I/O operations in test setup/teardown context".to_string(),
            });
        }
        
        // Look for batch processing patterns
        if self.has_batch_processing_pattern(patterns, contexts) {
            correlations.push(PatternCorrelation {
                pattern_group: patterns.to_vec(),
                correlation_type: CorrelationType::BatchProcessing,
                confidence_boost: 0.8,
                severity_reduction: 0.7,
                explanation: "I/O in batch processing context - expected pattern".to_string(),
            });
        }
        
        // Look for error handling patterns
        if self.has_error_handling_pattern(patterns, contexts) {
            correlations.push(PatternCorrelation {
                pattern_group: patterns.to_vec(),
                correlation_type: CorrelationType::ErrorHandling,
                confidence_boost: 0.7,
                severity_reduction: 0.6,
                explanation: "I/O in error handling context - often acceptable".to_string(),
            });
        }
        
        correlations
    }
    
    fn has_test_fixture_pattern(
        &self,
        patterns: &[PerformanceAntiPattern],
        contexts: &[PatternContext]
    ) -> bool {
        // Check for I/O operations in test context with setup/teardown intent
        patterns.iter().any(|p| matches!(p, PerformanceAntiPattern::InefficientIO { .. })) &&
        contexts.iter().any(|c| 
            matches!(c.module_type, ModuleType::Test) &&
            matches!(c.function_intent, FunctionIntent::Setup | FunctionIntent::Teardown)
        )
    }
}
```

### Architecture Changes

#### Modified Components
- `src/performance/mod.rs`: Integration point for smart detection
- `src/performance/io_detector.rs`: Enhanced with context awareness
- `src/analyzers/rust.rs`: Integration with smart performance detection
- `src/priority/unified_scorer.rs`: Context-aware scoring for performance debt

#### New Components
- `src/performance/context/`: Complete context analysis framework
- `src/performance/smart_detector.rs`: Main smart detection orchestrator
- `src/performance/pattern_correlator.rs`: Multi-pattern analysis engine
- `src/config.rs`: Configuration for smart pattern matching

#### Configuration Integration
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartPerformanceConfig {
    pub enabled: bool,
    pub context_analysis_enabled: bool,
    pub pattern_correlation_enabled: bool,
    
    // Confidence thresholds
    pub high_confidence_threshold: f64,     // 0.9
    pub production_confidence_threshold: f64, // 0.7
    pub test_confidence_threshold: f64,     // 0.5
    pub utility_confidence_threshold: f64,  // 0.8
    pub base_confidence_threshold: f64,     // 0.6
    
    // Context weights
    pub context_weights: ContextWeights,
    
    // Pattern-specific settings
    pub ignore_test_fixtures: bool,         // true
    pub reduce_test_severity: bool,         // true
    pub boost_critical_paths: bool,         // true
    
    // Custom patterns
    pub custom_setup_patterns: Vec<String>,
    pub custom_teardown_patterns: Vec<String>,
    pub custom_ignore_patterns: Vec<String>,
}
```

## Dependencies

### Prerequisites
- **Spec 41**: Test Performance as Tech Debt
  - Provides foundation for test-aware performance analysis
  - Required for understanding test file context handling
  
- **Spec 35**: Debt Pattern Unified Scoring Integration
  - Provides the scoring framework for integrating smart detection
  - Required for priority calculation and debt aggregation

- **Spec 28**: Security Patterns Detection
  - Establishes pattern for multi-dimensional analysis with context
  - Provides architectural patterns for context-aware detection

### Affected Components
- `src/performance/`: Core performance detection modules
- `src/analyzers/rust.rs`: AST analysis integration
- `src/priority/unified_scorer.rs`: Scoring system integration
- `src/config.rs`: Configuration system extensions
- `src/main.rs`: CLI integration for smart detection flags

### External Dependencies
- No new external crates required
- Leverages existing syn, serde, and im dependencies
- Uses existing AST parsing and call graph infrastructure

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_module_classification_test_files() {
        let classifier = ModuleClassifier::new();
        
        assert_eq!(
            classifier.classify_module(Path::new("tests/integration_test.rs")),
            ModuleType::Test
        );
        assert_eq!(
            classifier.classify_module(Path::new("src/lib_test.rs")),
            ModuleType::Test
        );
        assert_eq!(
            classifier.classify_module(Path::new("tests/fixtures/data.rs")),
            ModuleType::Test
        );
    }
    
    #[test]
    fn test_function_intent_classification_setup() {
        let classifier = IntentClassifier::new();
        let function = create_test_function("setup_test_environment");
        
        let intent = classifier.classify_function_intent(&function, &CallGraph::new());
        assert_eq!(intent, FunctionIntent::Setup);
    }
    
    #[test]
    fn test_smart_detection_filters_test_fixtures() {
        let source = r#"
            #[cfg(test)]
            mod tests {
                use tempfile::TempDir;
                
                #[test]
                fn test_file_processing() {
                    let temp_dir = TempDir::new().unwrap();
                    
                    // This should be recognized as test fixture setup
                    for i in 0..5 {
                        let test_file = temp_dir.path().join(format!("test_{}.rs", i));
                        std::fs::write(&test_file, "test content").unwrap();
                    }
                    
                    // Test the actual functionality
                    process_files(&temp_dir.path());
                }
            }
        "#;
        
        let file = syn::parse_str::<File>(source).unwrap();
        let detector = SmartPerformanceDetector::new();
        let issues = detector.detect_with_context(&file, Path::new("tests/file_test.rs"), &CallGraph::new());
        
        // Should detect the I/O pattern but classify it as test fixture with low severity
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].context.module_type, ModuleType::Test));
        assert!(matches!(issues[0].context.function_intent, FunctionIntent::Setup));
        assert!(issues[0].adjusted_severity <= Priority::Low);
        assert!(issues[0].reasoning.contains("test fixture"));
    }
    
    #[test]
    fn test_production_io_maintains_high_severity() {
        let source = r#"
            pub fn process_user_requests(requests: &[Request]) -> Vec<Response> {
                let mut responses = Vec::new();
                for request in requests {
                    // This should be flagged as high-severity blocking I/O
                    let data = std::fs::read_to_string(&request.file_path).unwrap();
                    responses.push(process_data(&data));
                }
                responses
            }
        "#;
        
        let file = syn::parse_str::<File>(source).unwrap();
        let detector = SmartPerformanceDetector::new();
        let issues = detector.detect_with_context(&file, Path::new("src/request_processor.rs"), &CallGraph::new());
        
        // Should detect high-severity performance issue in production code
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].context.module_type, ModuleType::Production));
        assert!(issues[0].adjusted_severity >= Priority::High);
        assert!(issues[0].reasoning.contains("production"));
    }
    
    #[test]
    fn test_confidence_scoring() {
        let pattern = PerformanceAntiPattern::InefficientIO {
            io_pattern: IOPattern::SyncInLoop,
            batching_opportunity: true,
            async_opportunity: true,
            location: SourceLocation::default(),
        };
        
        let test_context = PatternContext {
            module_type: ModuleType::Test,
            function_intent: FunctionIntent::Setup,
            performance_sensitivity: PerformanceSensitivity::Irrelevant,
            business_criticality: BusinessCriticality::Development,
            architectural_pattern: Some(ArchitecturalPattern::TestFixture),
            confidence: 0.9,
        };
        
        let production_context = PatternContext {
            module_type: ModuleType::Production,
            function_intent: FunctionIntent::BusinessLogic,
            performance_sensitivity: PerformanceSensitivity::High,
            business_criticality: BusinessCriticality::Critical,
            architectural_pattern: None,
            confidence: 0.9,
        };
        
        let detector = SmartPerformanceDetector::new();
        
        // Test context should be filtered out or receive very low severity
        assert!(!detector.should_report_pattern(&pattern, &test_context, 0.9) ||
                detector.severity_adjuster.adjust_severity(&pattern, &test_context, 0.9) <= Priority::Low);
        
        // Production context should maintain high severity
        assert!(detector.should_report_pattern(&pattern, &production_context, 0.9));
        assert!(detector.severity_adjuster.adjust_severity(&pattern, &production_context, 0.9) >= Priority::High);
    }
}
```

### Integration Tests

```rust
// tests/smart_performance_integration.rs
#[test]
fn test_end_to_end_smart_detection() {
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/mixed_codebase", "--smart-performance"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should have reduced false positives from test files
    assert!(!stdout.contains("test fixture") || stdout.contains("(Test performance debt - lower priority)"));
    
    // Should still detect production performance issues
    assert!(stdout.contains("PERFORMANCE"));
    assert!(stdout.contains("production"));
}

#[test]
fn test_configuration_impact() {
    // Test with strict configuration
    let output_strict = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/test_heavy_codebase", "--config", "tests/configs/strict_smart.toml"])
        .output()
        .expect("Failed to execute debtmap");
    
    // Test with lenient configuration  
    let output_lenient = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/test_heavy_codebase", "--config", "tests/configs/lenient_smart.toml"])
        .output()
        .expect("Failed to execute debtmap");
    
    let strict_issues = count_performance_issues(&output_strict.stdout);
    let lenient_issues = count_performance_issues(&output_lenient.stdout);
    
    // Strict configuration should report more issues
    assert!(strict_issues > lenient_issues);
}
```

### Performance Tests

```rust
#[test]
fn test_smart_detection_performance_impact() {
    use std::time::Instant;
    
    let start = Instant::now();
    let baseline_output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/large_codebase"])
        .output()
        .expect("Failed to execute baseline");
    let baseline_duration = start.elapsed();
    
    let start = Instant::now();
    let smart_output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/large_codebase", "--smart-performance"])
        .output()
        .expect("Failed to execute smart detection");
    let smart_duration = start.elapsed();
    
    // Smart detection should add less than 15% overhead
    let overhead_ratio = smart_duration.as_secs_f64() / baseline_duration.as_secs_f64();
    assert!(overhead_ratio < 1.15, "Smart detection overhead too high: {:.2}%", (overhead_ratio - 1.0) * 100.0);
}
```

## Documentation Requirements

### Code Documentation
- Comprehensive rustdoc for all context analysis algorithms
- Examples of pattern matching rules and their rationale
- Performance characteristics of smart detection components
- Configuration options and their impact on detection accuracy

### User Documentation
```markdown
## Smart Performance Detection

Debtmap's smart performance detection reduces false positives by understanding code context and intent:

### Context-Aware Analysis

**Module Classification**
- Test files receive reduced severity for performance issues
- Benchmark and example code filtered appropriately
- Production code maintains full sensitivity

**Function Intent Recognition**
- Setup/teardown functions are identified and deprioritized
- Business logic functions receive appropriate scrutiny
- I/O wrapper functions are expected to perform I/O operations

**Framework Pattern Detection**
- Test fixture patterns (file creation, mocking) are recognized
- Batch processing patterns receive context-appropriate analysis
- Error handling I/O operations are assessed differently

### Configuration

Enable smart detection:
```bash
debtmap analyze . --smart-performance
```

Configure detection sensitivity:
```toml
[smart_performance]
enabled = true
ignore_test_fixtures = true
reduce_test_severity = true
production_confidence_threshold = 0.7
test_confidence_threshold = 0.5

[smart_performance.context_weights]
module_type_weight = 1.0
function_intent_weight = 0.8
business_criticality_weight = 1.2
performance_sensitivity_weight = 1.5
```

### Output Format

Smart analysis adds context information to performance issues:

```
🚀 PERFORMANCE ANALYSIS (Smart Detection)
├─ Issue: Blocking I/O in loop
├─ Context: Test fixture setup (87% confidence)
├─ Severity: Reduced from High to Low  
├─ Reasoning: I/O operations in test setup context are typically acceptable
└─ Recommendation: Consider batching if test performance is critical

🎯 CONTEXT-SPECIFIC RECOMMENDATIONS
├─ Production Code: Implement async I/O or batching
├─ Test Code: Acceptable pattern, optimize only if tests are slow
└─ Utility Code: Consider caching or lazy loading patterns
```
```

### Architecture Documentation
Update ARCHITECTURE.md with smart performance detection flow and context analysis architecture.

## Implementation Notes

### Phased Implementation
1. **Phase 1**: Context analysis framework and module classification
2. **Phase 2**: Function intent analysis and basic pattern correlation
3. **Phase 3**: Severity adjustment engine and smart filtering
4. **Phase 4**: Pattern correlation and multi-pattern analysis
5. **Phase 5**: Configuration system and performance optimization

### Edge Cases to Consider
- Ambiguous context (code that could be test or production)
- Mixed-purpose functions (setup that also does business logic)
- Framework-specific patterns not covered by base detection
- Custom domain patterns requiring user configuration

### Pattern Detection Accuracy
- Conservative approach: prefer false negatives over false positives
- High confidence thresholds for context classification
- Graceful degradation when context is uncertain
- User feedback mechanism for pattern classification tuning

## Usage Examples

### Basic Smart Detection
```bash
# Enable smart performance detection
debtmap analyze . --smart-performance

# Show detailed context reasoning
debtmap analyze . --smart-performance --detailed

# Use custom configuration
debtmap analyze . --smart-performance --config smart-config.toml
```

### Configuration Examples
```toml
# Conservative configuration - fewer false positives
[smart_performance]
enabled = true
ignore_test_fixtures = true
reduce_test_severity = true
production_confidence_threshold = 0.8
test_confidence_threshold = 0.3

# Aggressive configuration - more sensitive detection
[smart_performance]
enabled = true
ignore_test_fixtures = false
reduce_test_severity = false
production_confidence_threshold = 0.6
test_confidence_threshold = 0.7
```

### Framework-Specific Patterns
```toml
# Custom patterns for specific frameworks
[smart_performance.custom_patterns]
setup_patterns = ["before_each", "given", "arrange", "fixture_"]
teardown_patterns = ["after_each", "cleanup_", "reset_"]
ignore_patterns = ["mock_", "stub_", "fake_"]
```

## Expected Impact

After implementation:

1. **Reduced False Positives**: 70%+ reduction in irrelevant performance warnings from test code and utilities
2. **Maintained Sensitivity**: Real performance issues in production code continue to be detected with high accuracy
3. **Better User Experience**: Developers receive actionable recommendations that acknowledge legitimate patterns
4. **Context-Aware Guidance**: Recommendations tailored to code context and architectural patterns
5. **Improved Adoption**: Reduced noise leads to better tool adoption and trust in recommendations

This specification addresses the core issue of context-blind pattern detection by implementing sophisticated semantic analysis that understands code intent and purpose, leading to more accurate and actionable performance debt detection.

## Migration and Compatibility

- **Breaking Changes**: None - smart detection is opt-in via CLI flag
- **Configuration Migration**: New configuration section with sensible defaults  
- **Output Compatibility**: Enhanced output maintains existing structure with additional context information
- **API Stability**: New functionality integrates with existing performance detection pipeline

Smart pattern matching provides a significant accuracy improvement while maintaining full backward compatibility with existing workflows and configurations.