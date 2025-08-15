---
number: 27
title: Context-Aware Complexity Scoring
category: feature
priority: high
status: draft
dependencies: [24, 26]
created: 2025-08-15
---

# Specification 27: Context-Aware Complexity Scoring

**Category**: feature
**Priority**: high
**Status**: draft
**Dependencies**: [24 - Refined Risk Scoring Methodology, 26 - Language-Specific Call Graph Architecture]

## Context

Current complexity scoring treats all code patterns equally, applying the same cyclomatic and cognitive complexity metrics regardless of the underlying purpose or domain. This approach often misrepresents the true complexity burden:

- **State machines** with many branches represent necessary complexity for handling distinct states
- **Protocol parsers** with extensive pattern matching reflect required message handling
- **Algorithm implementations** (sorting, searching) have inherent complexity that's well-understood and tested
- **Configuration parsing** with validation rules represents essential business logic
- **Business rule engines** necessarily encode complex domain knowledge

Meanwhile, accidental complexity from poor design choices (God objects, deeply nested conditionals without clear purpose, excessive parameter lists) should be weighted more heavily as genuine technical debt.

The current system penalizes a well-designed state machine with 15 clear states equally to a poorly designed function with 15 nested if-statements for unrelated concerns. This leads to false positives where well-architected but necessarily complex code receives the same priority as genuinely problematic code.

## Objective

Implement context-aware complexity scoring that distinguishes between necessary (inherent) and accidental complexity by:

1. **Pattern Detection**: Automatically identify common complexity patterns like state machines, parsers, algorithms, and business rules
2. **Complexity Adjustment**: Apply context-specific scoring factors that reduce penalties for necessary complexity while maintaining or increasing penalties for accidental complexity  
3. **Enhanced Recommendations**: Provide pattern-specific guidance that acknowledges the legitimate need for complexity in certain contexts
4. **Risk Calibration**: Integrate with the existing risk scoring system to provide more accurate technical debt assessments

## Requirements

### Functional Requirements

1. **Pattern Recognition System**
   - Detect state machine patterns (enum/match combinations, state variables)
   - Identify parser implementations (recursive descent, protocol handling)
   - Recognize algorithm implementations (sorting, searching, mathematical)
   - Find business rule engines (validation chains, decision trees)
   - Detect configuration handling (serialization, validation, parsing)

2. **Context-Specific Complexity Adjustment**
   - **State Machine Factor**: 0.6-0.8 multiplier for recognized state machine patterns
   - **Parser Factor**: 0.7-0.8 multiplier for protocol/format parsing code
   - **Algorithm Factor**: 0.5-0.7 multiplier for well-known algorithm implementations
   - **Business Rule Factor**: 0.8-0.9 multiplier for business logic concentration
   - **Accidental Complexity Boost**: 1.2-1.5 multiplier for anti-patterns

3. **Pattern-Aware Recommendations**
   - State machines: Suggest state transition testing, state diagram documentation
   - Parsers: Recommend parser combinator refactoring, input validation testing
   - Algorithms: Suggest property-based testing, complexity analysis documentation
   - Business rules: Recommend rule extraction, domain modeling improvements
   - Accidental complexity: Standard refactoring recommendations with emphasis on design patterns

4. **Risk Score Integration**
   - Adjust complexity contribution to overall risk score based on context
   - Maintain backward compatibility with existing risk calculation pipeline
   - Preserve relative ordering for genuinely problematic code
   - Enhance accuracy for well-designed but complex code

### Non-Functional Requirements

1. **Performance**
   - Pattern detection adds <20% overhead to analysis time
   - Efficient pattern matching using AST traversal visitors
   - Cacheable pattern recognition results
   - Incremental pattern detection for large codebases

2. **Accuracy**
   - >85% precision for pattern detection (low false positives)
   - >75% recall for pattern detection (catches most instances)
   - Configurable confidence thresholds for pattern recognition
   - Fallback to standard complexity scoring for uncertain cases

3. **Extensibility**
   - Plugin architecture for adding new pattern detectors
   - Language-specific pattern implementations building on spec 26 architecture
   - Configurable adjustment factors per pattern type
   - Support for custom pattern definitions via configuration

4. **Maintainability**
   - Clear separation between pattern detection and scoring adjustment
   - Comprehensive test coverage for pattern recognition
   - Documented pattern detection algorithms and their rationale
   - Readable, well-structured code following functional programming principles

## Acceptance Criteria

- [ ] **State Machine Detection**: Correctly identifies enum-based state machines and applies 0.6-0.8 complexity multiplier
- [ ] **Parser Recognition**: Detects recursive descent parsers, protocol handlers, and format parsers with 0.7-0.8 multiplier
- [ ] **Algorithm Identification**: Recognizes sorting, searching, and mathematical algorithms with 0.5-0.7 multiplier
- [ ] **Business Rule Detection**: Identifies validation chains and business logic with 0.8-0.9 multiplier
- [ ] **Anti-Pattern Detection**: Finds God objects, deep nesting without clear purpose with 1.2-1.5 multiplier
- [ ] **Pattern-Specific Recommendations**: Provides contextually appropriate guidance for each pattern type
- [ ] **Risk Integration**: Adjusted complexity scores properly influence overall risk calculations
- [ ] **Performance**: Pattern detection completes within 20% of baseline analysis time
- [ ] **Configuration**: All adjustment factors and detection thresholds are configurable
- [ ] **Backward Compatibility**: Existing complexity scores remain stable when context-aware scoring is disabled

## Technical Details

### Implementation Approach

#### 1. Pattern Detection Architecture (`src/complexity/patterns/`)

```rust
/// Core pattern detection traits and types
pub mod patterns {
    use crate::core::ast::AstNode;
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ComplexityPattern {
        StateMachine {
            state_count: usize,
            transition_complexity: f64,
            confidence: f64,
        },
        Parser {
            parser_type: ParserType,
            grammar_complexity: f64,
            confidence: f64,
        },
        Algorithm {
            algorithm_type: AlgorithmType,
            time_complexity: String,
            confidence: f64,
        },
        BusinessRule {
            rule_count: usize,
            rule_complexity: f64,
            confidence: f64,
        },
        AccidentalComplexity {
            anti_pattern: AntiPatternType,
            severity: f64,
            confidence: f64,
        },
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum ParserType {
        RecursiveDescent,
        ProtocolHandler,
        FormatParser,
        Tokenizer,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum AlgorithmType {
        Sorting,
        Searching,
        GraphTraversal,
        Mathematical,
        Cryptographic,
    }
    
    #[derive(Debug, Clone, PartialEq)]
    pub enum AntiPatternType {
        GodObject,
        DeepNesting,
        LongParameterList,
        SwitchStatement,
        LongMethod,
    }
    
    pub trait PatternDetector {
        fn detect_patterns(&self, ast: &AstNode) -> Vec<ComplexityPattern>;
        fn confidence_threshold(&self) -> f64;
        fn pattern_type(&self) -> &'static str;
    }
}
```

#### 2. State Machine Pattern Detector (`src/complexity/patterns/state_machine.rs`)

```rust
pub struct StateMachineDetector {
    min_states: usize,
    confidence_threshold: f64,
}

impl PatternDetector for StateMachineDetector {
    fn detect_patterns(&self, ast: &AstNode) -> Vec<ComplexityPattern> {
        let mut patterns = Vec::new();
        
        // Look for enum definitions with match expressions
        if let Some(state_enum) = self.find_state_enum(ast) {
            let match_expressions = self.find_related_matches(ast, &state_enum);
            
            if !match_expressions.is_empty() {
                let state_count = state_enum.variants.len();
                let transition_complexity = self.calculate_transition_complexity(&match_expressions);
                let confidence = self.calculate_confidence(state_count, &match_expressions);
                
                if confidence >= self.confidence_threshold && state_count >= self.min_states {
                    patterns.push(ComplexityPattern::StateMachine {
                        state_count,
                        transition_complexity,
                        confidence,
                    });
                }
            }
        }
        
        patterns
    }
}

impl StateMachineDetector {
    fn find_state_enum(&self, ast: &AstNode) -> Option<EnumDefinition> {
        // Heuristics for state enum detection:
        // - Enum name contains "State", "Status", "Phase", "Mode"
        // - Variants represent distinct states
        // - Used in pattern matching
        ast.find_enums()
            .into_iter()
            .find(|enum_def| {
                self.is_likely_state_enum(enum_def) && 
                enum_def.variants.len() >= self.min_states
            })
    }
    
    fn is_likely_state_enum(&self, enum_def: &EnumDefinition) -> bool {
        let state_keywords = ["state", "status", "phase", "mode", "stage"];
        let name = enum_def.name.to_lowercase();
        
        state_keywords.iter().any(|keyword| name.contains(keyword)) ||
        self.has_state_like_variants(enum_def)
    }
    
    fn has_state_like_variants(&self, enum_def: &EnumDefinition) -> bool {
        let state_patterns = [
            "init", "start", "begin", "ready", "active", "running",
            "paused", "stopped", "end", "complete", "error", "failed"
        ];
        
        let matching_variants = enum_def.variants
            .iter()
            .filter(|variant| {
                let name = variant.name.to_lowercase();
                state_patterns.iter().any(|pattern| name.contains(pattern))
            })
            .count();
            
        matching_variants >= (enum_def.variants.len() / 2)
    }
    
    fn calculate_transition_complexity(&self, matches: &[MatchExpression]) -> f64 {
        matches.iter()
            .map(|match_expr| {
                // Calculate complexity of state transitions
                match_expr.arms.iter()
                    .map(|arm| self.calculate_arm_complexity(arm))
                    .sum::<f64>()
            })
            .sum::<f64>() / matches.len() as f64
    }
    
    fn calculate_confidence(&self, state_count: usize, matches: &[MatchExpression]) -> f64 {
        let base_confidence = 0.7;
        let state_factor = (state_count as f64 / 10.0).min(1.0) * 0.2;
        let match_factor = (matches.len() as f64 / 3.0).min(1.0) * 0.1;
        
        (base_confidence + state_factor + match_factor).min(1.0)
    }
}
```

#### 3. Parser Pattern Detector (`src/complexity/patterns/parser.rs`)

```rust
pub struct ParserDetector {
    confidence_threshold: f64,
}

impl PatternDetector for ParserDetector {
    fn detect_patterns(&self, ast: &AstNode) -> Vec<ComplexityPattern> {
        let mut patterns = Vec::new();
        
        // Detect recursive descent patterns
        if let Some(parser_pattern) = self.detect_recursive_descent(ast) {
            patterns.push(parser_pattern);
        }
        
        // Detect protocol handlers
        if let Some(protocol_pattern) = self.detect_protocol_handler(ast) {
            patterns.push(protocol_pattern);
        }
        
        // Detect format parsers (JSON, XML, etc.)
        if let Some(format_pattern) = self.detect_format_parser(ast) {
            patterns.push(format_pattern);
        }
        
        patterns
    }
}

impl ParserDetector {
    fn detect_recursive_descent(&self, ast: &AstNode) -> Option<ComplexityPattern> {
        // Look for recursive function patterns typical of parsers
        let recursive_functions = self.find_recursive_functions(ast);
        let parsing_keywords = ["parse", "expect", "consume", "peek", "advance"];
        
        let parser_functions: Vec<_> = recursive_functions
            .into_iter()
            .filter(|func| {
                parsing_keywords.iter().any(|keyword| {
                    func.name.to_lowercase().contains(keyword)
                }) || self.has_parsing_patterns(func)
            })
            .collect();
            
        if parser_functions.len() >= 2 {
            let grammar_complexity = self.estimate_grammar_complexity(&parser_functions);
            let confidence = self.calculate_parser_confidence(&parser_functions);
            
            if confidence >= self.confidence_threshold {
                return Some(ComplexityPattern::Parser {
                    parser_type: ParserType::RecursiveDescent,
                    grammar_complexity,
                    confidence,
                });
            }
        }
        
        None
    }
    
    fn has_parsing_patterns(&self, func: &FunctionDefinition) -> bool {
        // Look for common parser patterns:
        // - Error handling for unexpected tokens
        // - Position/cursor tracking
        // - Lookahead operations
        func.body.contains_patterns(&[
            "unexpected", "expected", "position", "cursor", 
            "lookahead", "peek", "token", "syntax"
        ])
    }
    
    fn detect_protocol_handler(&self, ast: &AstNode) -> Option<ComplexityPattern> {
        // Look for network protocol or message handling patterns
        let protocol_indicators = [
            "message", "packet", "frame", "header", "payload",
            "protocol", "decode", "encode", "serialize", "deserialize"
        ];
        
        let protocol_functions = ast.find_functions()
            .into_iter()
            .filter(|func| {
                protocol_indicators.iter().any(|indicator| {
                    func.name.to_lowercase().contains(indicator)
                })
            })
            .collect::<Vec<_>>();
            
        if protocol_functions.len() >= 3 {
            let complexity = self.calculate_protocol_complexity(&protocol_functions);
            let confidence = 0.8; // High confidence for protocol patterns
            
            return Some(ComplexityPattern::Parser {
                parser_type: ParserType::ProtocolHandler,
                grammar_complexity: complexity,
                confidence,
            });
        }
        
        None
    }
}
```

#### 4. Algorithm Pattern Detector (`src/complexity/patterns/algorithm.rs`)

```rust
pub struct AlgorithmDetector {
    confidence_threshold: f64,
}

impl PatternDetector for AlgorithmDetector {
    fn detect_patterns(&self, ast: &AstNode) -> Vec<ComplexityPattern> {
        let mut patterns = Vec::new();
        
        // Detect sorting algorithms
        patterns.extend(self.detect_sorting_algorithms(ast));
        
        // Detect search algorithms
        patterns.extend(self.detect_search_algorithms(ast));
        
        // Detect graph algorithms
        patterns.extend(self.detect_graph_algorithms(ast));
        
        // Detect mathematical algorithms
        patterns.extend(self.detect_mathematical_algorithms(ast));
        
        patterns
    }
}

impl AlgorithmDetector {
    fn detect_sorting_algorithms(&self, ast: &AstNode) -> Vec<ComplexityPattern> {
        let sort_functions = ast.find_functions()
            .into_iter()
            .filter(|func| {
                let name = func.name.to_lowercase();
                name.contains("sort") || 
                self.has_sorting_patterns(func)
            })
            .collect::<Vec<_>>();
            
        sort_functions.into_iter()
            .filter_map(|func| {
                let algorithm_type = self.identify_sort_algorithm(&func)?;
                let complexity = self.get_known_complexity(&algorithm_type);
                
                Some(ComplexityPattern::Algorithm {
                    algorithm_type: AlgorithmType::Sorting,
                    time_complexity: complexity.to_string(),
                    confidence: 0.9, // High confidence for well-known algorithms
                })
            })
            .collect()
    }
    
    fn has_sorting_patterns(&self, func: &FunctionDefinition) -> bool {
        // Look for sorting patterns:
        // - Comparison operations in loops
        // - Swapping elements
        // - Partitioning logic
        func.has_nested_loops() && 
        func.has_comparison_operations() &&
        (func.has_swapping_pattern() || func.has_partitioning_pattern())
    }
    
    fn identify_sort_algorithm(&self, func: &FunctionDefinition) -> Option<&'static str> {
        // Identify specific sorting algorithms by their patterns
        if func.has_quicksort_pattern() {
            Some("quicksort")
        } else if func.has_mergesort_pattern() {
            Some("mergesort")
        } else if func.has_heapsort_pattern() {
            Some("heapsort")
        } else if func.has_bubblesort_pattern() {
            Some("bubblesort")
        } else {
            None
        }
    }
    
    fn detect_search_algorithms(&self, ast: &AstNode) -> Vec<ComplexityPattern> {
        let search_functions = ast.find_functions()
            .into_iter()
            .filter(|func| {
                let name = func.name.to_lowercase();
                name.contains("search") || name.contains("find") ||
                self.has_search_patterns(func)
            })
            .collect::<Vec<_>>();
            
        search_functions.into_iter()
            .filter_map(|func| {
                if func.has_binary_search_pattern() {
                    Some(ComplexityPattern::Algorithm {
                        algorithm_type: AlgorithmType::Searching,
                        time_complexity: "O(log n)".to_string(),
                        confidence: 0.95,
                    })
                } else if func.has_linear_search_pattern() {
                    Some(ComplexityPattern::Algorithm {
                        algorithm_type: AlgorithmType::Searching,
                        time_complexity: "O(n)".to_string(),
                        confidence: 0.9,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}
```

#### 5. Complexity Adjustment Calculator (`src/complexity/context_aware.rs`)

```rust
pub struct ContextAwareComplexityCalculator {
    pattern_detectors: Vec<Box<dyn PatternDetector>>,
    adjustment_config: AdjustmentConfig,
}

#[derive(Debug, Clone)]
pub struct AdjustmentConfig {
    pub state_machine_factor: (f64, f64),    // (min, max) = (0.6, 0.8)
    pub parser_factor: (f64, f64),           // (min, max) = (0.7, 0.8)
    pub algorithm_factor: (f64, f64),        // (min, max) = (0.5, 0.7)
    pub business_rule_factor: (f64, f64),    // (min, max) = (0.8, 0.9)
    pub accidental_complexity_factor: (f64, f64), // (min, max) = (1.2, 1.5)
    pub confidence_weight: f64,              // How much confidence affects the adjustment
}

impl ContextAwareComplexityCalculator {
    pub fn calculate_adjusted_complexity(
        &self, 
        base_complexity: &ComplexityMetrics,
        ast: &AstNode
    ) -> AdjustedComplexityMetrics {
        let patterns = self.detect_all_patterns(ast);
        let adjustment = self.calculate_adjustment_factor(&patterns);
        
        AdjustedComplexityMetrics {
            original_cyclomatic: base_complexity.cyclomatic,
            original_cognitive: base_complexity.cognitive,
            adjusted_cyclomatic: base_complexity.cyclomatic * adjustment.cyclomatic_factor,
            adjusted_cognitive: base_complexity.cognitive * adjustment.cognitive_factor,
            detected_patterns: patterns,
            adjustment_factor: adjustment,
            confidence: self.calculate_overall_confidence(&patterns),
        }
    }
    
    fn detect_all_patterns(&self, ast: &AstNode) -> Vec<ComplexityPattern> {
        self.pattern_detectors
            .iter()
            .flat_map(|detector| detector.detect_patterns(ast))
            .collect()
    }
    
    fn calculate_adjustment_factor(&self, patterns: &[ComplexityPattern]) -> AdjustmentFactor {
        if patterns.is_empty() {
            return AdjustmentFactor::neutral();
        }
        
        // Find the most confident pattern
        let primary_pattern = patterns
            .iter()
            .max_by(|a, b| a.confidence().partial_cmp(&b.confidence()).unwrap())
            .unwrap();
            
        match primary_pattern {
            ComplexityPattern::StateMachine { confidence, .. } => {
                self.interpolate_factor(
                    self.adjustment_config.state_machine_factor,
                    *confidence
                )
            }
            ComplexityPattern::Parser { confidence, .. } => {
                self.interpolate_factor(
                    self.adjustment_config.parser_factor,
                    *confidence
                )
            }
            ComplexityPattern::Algorithm { confidence, .. } => {
                self.interpolate_factor(
                    self.adjustment_config.algorithm_factor,
                    *confidence
                )
            }
            ComplexityPattern::BusinessRule { confidence, .. } => {
                self.interpolate_factor(
                    self.adjustment_config.business_rule_factor,
                    *confidence
                )
            }
            ComplexityPattern::AccidentalComplexity { confidence, .. } => {
                self.interpolate_factor(
                    self.adjustment_config.accidental_complexity_factor,
                    *confidence
                )
            }
        }
    }
    
    fn interpolate_factor(&self, factor_range: (f64, f64), confidence: f64) -> AdjustmentFactor {
        let (min_factor, max_factor) = factor_range;
        let confidence_adjusted = confidence * self.adjustment_config.confidence_weight;
        let factor = min_factor + (max_factor - min_factor) * confidence_adjusted;
        
        AdjustmentFactor {
            cyclomatic_factor: factor,
            cognitive_factor: factor,
            reasoning: format!("Applied {} factor based on {:.1}% confidence", 
                             self.pattern_type_name(factor_range), confidence * 100.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdjustmentFactor {
    pub cyclomatic_factor: f64,
    pub cognitive_factor: f64,
    pub reasoning: String,
}

impl AdjustmentFactor {
    fn neutral() -> Self {
        Self {
            cyclomatic_factor: 1.0,
            cognitive_factor: 1.0,
            reasoning: "No patterns detected - using standard complexity scoring".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdjustedComplexityMetrics {
    pub original_cyclomatic: u32,
    pub original_cognitive: u32,
    pub adjusted_cyclomatic: f64,
    pub adjusted_cognitive: f64,
    pub detected_patterns: Vec<ComplexityPattern>,
    pub adjustment_factor: AdjustmentFactor,
    pub confidence: f64,
}
```

#### 6. Integration with Risk Scoring (`src/risk/context_aware_risk.rs`)

```rust
use crate::complexity::context_aware::AdjustedComplexityMetrics;
use crate::risk::evidence::RiskEvidence;

pub struct ContextAwareRiskCalculator {
    base_calculator: Box<dyn RiskCalculator>,
}

impl RiskCalculator for ContextAwareRiskCalculator {
    fn calculate_risk(&self, evidence: &RiskEvidence) -> RiskScore {
        let base_risk = self.base_calculator.calculate_risk(evidence);
        
        // Apply context-aware adjustments if available
        if let Some(adjusted_complexity) = &evidence.adjusted_complexity {
            self.apply_context_adjustments(base_risk, adjusted_complexity)
        } else {
            base_risk
        }
    }
}

impl ContextAwareRiskCalculator {
    fn apply_context_adjustments(
        &self,
        mut risk: RiskScore,
        adjusted: &AdjustedComplexityMetrics
    ) -> RiskScore {
        // Adjust complexity contribution to risk score
        let complexity_adjustment = adjusted.adjusted_cyclomatic / adjusted.original_cyclomatic as f64;
        
        risk.complexity_score *= complexity_adjustment;
        risk.overall_score = self.recalculate_overall_score(&risk);
        
        // Add context information to risk explanation
        risk.context = Some(RiskContext {
            detected_patterns: adjusted.detected_patterns.clone(),
            adjustment_reasoning: adjusted.adjustment_factor.reasoning.clone(),
            confidence: adjusted.confidence,
        });
        
        risk
    }
}

#[derive(Debug, Clone)]
pub struct RiskContext {
    pub detected_patterns: Vec<ComplexityPattern>,
    pub adjustment_reasoning: String,
    pub confidence: f64,
}
```

#### 7. Enhanced Recommendations (`src/recommendations/context_aware.rs`)

```rust
pub struct ContextAwareRecommendationEngine {
    pattern_advisors: HashMap<String, Box<dyn PatternAdvisor>>,
}

pub trait PatternAdvisor {
    fn generate_recommendations(&self, pattern: &ComplexityPattern, metrics: &AdjustedComplexityMetrics) -> Vec<Recommendation>;
}

pub struct StateMachineAdvisor;

impl PatternAdvisor for StateMachineAdvisor {
    fn generate_recommendations(&self, pattern: &ComplexityPattern, metrics: &AdjustedComplexityMetrics) -> Vec<Recommendation> {
        if let ComplexityPattern::StateMachine { state_count, transition_complexity, .. } = pattern {
            let mut recommendations = Vec::new();
            
            if *state_count > 10 {
                recommendations.push(Recommendation {
                    priority: RecommendationPriority::Medium,
                    category: RecommendationCategory::Architecture,
                    title: "Consider state machine decomposition".to_string(),
                    description: format!(
                        "State machine with {} states may benefit from hierarchical decomposition or state grouping",
                        state_count
                    ),
                    effort_estimate: EffortEstimate::Medium,
                    impact_estimate: ImpactEstimate::High,
                });
            }
            
            if *transition_complexity > 5.0 {
                recommendations.push(Recommendation {
                    priority: RecommendationPriority::High,
                    category: RecommendationCategory::Testing,
                    title: "Add state transition testing".to_string(),
                    description: "Complex state transitions require comprehensive testing of all valid and invalid transitions".to_string(),
                    effort_estimate: EffortEstimate::High,
                    impact_estimate: ImpactEstimate::High,
                });
            }
            
            recommendations.push(Recommendation {
                priority: RecommendationPriority::Low,
                category: RecommendationCategory::Documentation,
                title: "Document state machine design".to_string(),
                description: "Create state transition diagram and document valid state transitions and triggering events".to_string(),
                effort_estimate: EffortEstimate::Low,
                impact_estimate: ImpactEstimate::Medium,
            });
            
            recommendations
        } else {
            Vec::new()
        }
    }
}

pub struct AlgorithmAdvisor;

impl PatternAdvisor for AlgorithmAdvisor {
    fn generate_recommendations(&self, pattern: &ComplexityPattern, metrics: &AdjustedComplexityMetrics) -> Vec<Recommendation> {
        if let ComplexityPattern::Algorithm { algorithm_type, time_complexity, .. } = pattern {
            let mut recommendations = Vec::new();
            
            recommendations.push(Recommendation {
                priority: RecommendationPriority::Medium,
                category: RecommendationCategory::Testing,
                title: "Add property-based testing".to_string(),
                description: format!(
                    "Algorithm implementations benefit from property-based testing to verify correctness across input ranges. Time complexity: {}",
                    time_complexity
                ),
                effort_estimate: EffortEstimate::Medium,
                impact_estimate: ImpactEstimate::High,
            });
            
            if matches!(algorithm_type, AlgorithmType::Sorting) {
                recommendations.push(Recommendation {
                    priority: RecommendationPriority::Low,
                    category: RecommendationCategory::Performance,
                    title: "Consider using standard library".to_string(),
                    description: "Evaluate if standard library sort implementations would be more appropriate".to_string(),
                    effort_estimate: EffortEstimate::Low,
                    impact_estimate: ImpactEstimate::Medium,
                });
            }
            
            recommendations
        } else {
            Vec::new()
        }
    }
}
```

### Architecture Changes

#### Modified Components
- `src/complexity/mod.rs`: Add context-aware complexity calculation integration
- `src/complexity/patterns/`: New module for pattern detection (state machines, parsers, algorithms)
- `src/risk/evidence_calculator.rs`: Include adjusted complexity metrics in risk evidence
- `src/recommendations/mod.rs`: Integration with context-aware recommendation engine
- `src/analyzers/rust.rs`: Call context-aware complexity calculation during analysis

#### New Configuration Options
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAwareConfig {
    pub enabled: bool,
    pub pattern_detection: PatternDetectionConfig,
    pub adjustment_factors: AdjustmentConfig,
    pub confidence_thresholds: ConfidenceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternDetectionConfig {
    pub state_machine_min_states: usize,      // Default: 3
    pub parser_min_functions: usize,          // Default: 2
    pub algorithm_detection_enabled: bool,    // Default: true
    pub business_rule_detection_enabled: bool, // Default: true
    pub anti_pattern_detection_enabled: bool, // Default: true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceConfig {
    pub min_pattern_confidence: f64,          // Default: 0.7
    pub min_adjustment_confidence: f64,       // Default: 0.8
    pub uncertainty_fallback: bool,           // Default: true (fall back to standard scoring)
}
```

### Data Flow Integration

```
Files → Parse → AST → [Pattern Detection] → [Context-Aware Complexity] → Risk Analysis → Recommendations
                ↓              ↓                   ↓
         Standard Complexity → Adjusted Complexity → Enhanced Risk Score
```

## Dependencies

### Prerequisites
- **Spec 24**: Refined Risk Scoring Methodology
  - Provides the risk calculation framework and evidence structure
  - Required for integrating adjusted complexity into risk scores

- **Spec 26**: Language-Specific Call Graph Architecture  
  - Establishes pattern for language-specific analysis implementations
  - Provides architectural foundation for extending pattern detection to other languages

### Affected Components
- `src/complexity/`: Core complexity calculation module
- `src/risk/evidence_calculator.rs`: Risk evidence collection
- `src/analyzers/rust.rs`: Rust-specific AST analysis
- `src/recommendations/`: Recommendation generation
- Configuration system for adjustment factors and thresholds

### External Dependencies
- No new external crates required
- Uses existing syn, serde, and im dependencies
- Leverages existing AST parsing and traversal infrastructure

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_state_machine_detection() {
        let source = r#"
            enum ConnectionState {
                Connecting,
                Connected,
                Disconnected,
                Error,
            }
            
            fn handle_connection(state: ConnectionState) -> ConnectionState {
                match state {
                    ConnectionState::Connecting => {
                        // Complex connection logic
                        ConnectionState::Connected
                    }
                    ConnectionState::Connected => {
                        // Handle connected state
                        state
                    }
                    // ... other arms
                }
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = StateMachineDetector::new();
        let patterns = detector.detect_patterns(&ast);
        
        assert_eq!(patterns.len(), 1);
        if let ComplexityPattern::StateMachine { state_count, confidence, .. } = &patterns[0] {
            assert_eq!(*state_count, 4);
            assert!(*confidence > 0.7);
        } else {
            panic!("Expected state machine pattern");
        }
    }
    
    #[test]
    fn test_algorithm_detection() {
        let source = r#"
            fn quicksort(arr: &mut [i32]) {
                if arr.len() <= 1 {
                    return;
                }
                let pivot = partition(arr);
                quicksort(&mut arr[..pivot]);
                quicksort(&mut arr[pivot + 1..]);
            }
            
            fn partition(arr: &mut [i32]) -> usize {
                // Partitioning logic with complexity
                0
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = AlgorithmDetector::new();
        let patterns = detector.detect_patterns(&ast);
        
        assert!(!patterns.is_empty());
        if let ComplexityPattern::Algorithm { algorithm_type, time_complexity, .. } = &patterns[0] {
            assert_eq!(*algorithm_type, AlgorithmType::Sorting);
            assert!(time_complexity.contains("O(n log n)") || time_complexity.contains("O(n²)"));
        } else {
            panic!("Expected algorithm pattern");
        }
    }
    
    #[test]
    fn test_complexity_adjustment() {
        let base_complexity = ComplexityMetrics {
            cyclomatic: 15,
            cognitive: 20,
        };
        
        let patterns = vec![ComplexityPattern::StateMachine {
            state_count: 8,
            transition_complexity: 3.0,
            confidence: 0.9,
        }];
        
        let calculator = ContextAwareComplexityCalculator::new();
        let adjustment = calculator.calculate_adjustment_factor(&patterns);
        
        // State machine should get reduced penalty
        assert!(adjustment.cyclomatic_factor < 1.0);
        assert!(adjustment.cyclomatic_factor >= 0.6);
        assert!(adjustment.cyclomatic_factor <= 0.8);
    }
    
    #[test]
    fn test_anti_pattern_boost() {
        let source = r#"
            fn god_function(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32) -> Result<String, Error> {
                if a > 0 {
                    if b > 0 {
                        if c > 0 {
                            if d > 0 {
                                if e > 0 {
                                    // Deep nesting without clear purpose
                                    if f > 0 {
                                        if g > 0 {
                                            // More deeply nested logic
                                            Ok("result".to_string())
                                        } else {
                                            Err(Error::new())
                                        }
                                    } else {
                                        Err(Error::new())
                                    }
                                } else {
                                    Err(Error::new())
                                }
                            } else {
                                Err(Error::new())
                            }
                        } else {
                            Err(Error::new())
                        }
                    } else {
                        Err(Error::new())
                    }
                } else {
                    Err(Error::new())
                }
            }
        "#;
        
        let ast = parse_rust_source(source);
        let detector = AntiPatternDetector::new();
        let patterns = detector.detect_patterns(&ast);
        
        assert!(!patterns.is_empty());
        if let ComplexityPattern::AccidentalComplexity { anti_pattern, .. } = &patterns[0] {
            assert!(matches!(anti_pattern, AntiPatternType::DeepNesting | AntiPatternType::LongParameterList));
        }
        
        let calculator = ContextAwareComplexityCalculator::new();
        let adjustment = calculator.calculate_adjustment_factor(&patterns);
        
        // Anti-patterns should get increased penalty
        assert!(adjustment.cyclomatic_factor > 1.0);
        assert!(adjustment.cyclomatic_factor >= 1.2);
    }
}
```

### Integration Tests

```rust
// tests/context_aware_integration.rs
use std::process::Command;

#[test]
fn test_context_aware_complexity_end_to_end() {
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/state_machine_example", "--context-aware"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Verify that state machine patterns are detected and adjusted
    assert!(stdout.contains("State Machine"));
    assert!(stdout.contains("adjusted complexity"));
    assert!(stdout.contains("pattern-specific recommendations"));
}

#[test]
fn test_algorithm_detection_with_json_output() {
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/sorting_algorithms", "--context-aware", "--format", "json"])
        .output()
        .expect("Failed to execute debtmap");

    assert!(output.status.success());
    
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    
    // Verify algorithm patterns in JSON output
    assert!(json["analysis"]["context_aware"]["detected_patterns"].is_array());
    
    let patterns = json["analysis"]["context_aware"]["detected_patterns"].as_array().unwrap();
    assert!(!patterns.is_empty());
    
    // Check for algorithm pattern
    let has_algorithm = patterns.iter().any(|pattern| {
        pattern["pattern_type"].as_str() == Some("Algorithm")
    });
    assert!(has_algorithm);
}

#[test]
fn test_backward_compatibility() {
    // Test that disabling context-aware scoring produces same results as before
    let baseline_output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/complex_example"])
        .output()
        .expect("Failed to execute debtmap");

    let context_disabled_output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/complex_example", "--no-context-aware"])
        .output()
        .expect("Failed to execute debtmap");

    assert_eq!(baseline_output.stdout, context_disabled_output.stdout);
}
```

### Performance Tests

```rust
#[test]
fn test_pattern_detection_performance() {
    use std::time::Instant;
    
    let start = Instant::now();
    
    let output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/large_codebase", "--context-aware"])
        .output()
        .expect("Failed to execute debtmap");
    
    let context_aware_duration = start.elapsed();
    
    let start = Instant::now();
    
    let baseline_output = Command::new("./target/debug/debtmap")
        .args(&["analyze", "tests/fixtures/large_codebase"])
        .output()
        .expect("Failed to execute debtmap");
    
    let baseline_duration = start.elapsed();
    
    // Context-aware analysis should add less than 20% overhead
    let overhead_ratio = context_aware_duration.as_secs_f64() / baseline_duration.as_secs_f64();
    assert!(overhead_ratio < 1.2, "Pattern detection overhead too high: {:.2}%", (overhead_ratio - 1.0) * 100.0);
}
```

## Documentation Requirements

### Code Documentation
- Comprehensive rustdoc for all pattern detection algorithms
- Examples of each pattern type and their detection criteria
- Performance characteristics and complexity analysis for detectors
- Configuration options and their effects on detection accuracy

### User Documentation
```markdown
## Context-Aware Complexity Scoring

Debtmap can distinguish between necessary complexity (inherent to the problem domain) and accidental complexity (from poor design) using pattern detection:

### Supported Patterns

**State Machines**
- Detects enum-based state machines with pattern matching
- Reduces complexity penalty by 20-40% for well-designed state handling
- Recommends state transition testing and documentation

**Parsers and Protocol Handlers**  
- Identifies recursive descent parsers and protocol handling code
- Reduces complexity penalty by 20-30% for parsing logic
- Suggests parser combinator refactoring and input validation

**Algorithm Implementations**
- Recognizes sorting, searching, and mathematical algorithms
- Reduces complexity penalty by 30-50% for known algorithms
- Recommends property-based testing and complexity documentation

**Business Rules**
- Detects validation chains and business logic concentration
- Reduces complexity penalty by 10-20% for business rule encoding
- Suggests rule extraction and domain modeling improvements

### Configuration

Enable context-aware scoring:
```bash
debtmap analyze . --context-aware
```

Configure detection sensitivity:
```toml
[context_aware]
enabled = true
min_pattern_confidence = 0.7
state_machine_factor = [0.6, 0.8]
parser_factor = [0.7, 0.8]
algorithm_factor = [0.5, 0.7]
```

### Output Format

Context-aware analysis adds pattern information to standard output:

```
📊 COMPLEXITY ANALYSIS
├─ Original Complexity: 15 cyclomatic, 18 cognitive
├─ Detected Pattern: State Machine (87% confidence)
├─ Adjusted Complexity: 12 cyclomatic, 14.4 cognitive  
└─ Adjustment Reason: Applied state machine factor based on 87% confidence

🎯 PATTERN-SPECIFIC RECOMMENDATIONS
├─ Add state transition testing (High Priority)
├─ Document state machine design (Low Priority)
└─ Consider state grouping for >10 states (Medium Priority)
```
```

### Architecture Documentation
Update ARCHITECTURE.md with context-aware complexity calculation flow and pattern detection architecture.

## Implementation Notes

### Phased Implementation
1. **Phase 1**: Core pattern detection framework and state machine detection
2. **Phase 2**: Parser and algorithm pattern detection
3. **Phase 3**: Anti-pattern detection and complexity boost logic
4. **Phase 4**: Integration with risk scoring and recommendation systems
5. **Phase 5**: Performance optimization and configuration refinement

### Edge Cases to Consider
- Mixed patterns in single function (use highest confidence pattern)
- Low confidence detection (fall back to standard complexity scoring)
- Custom domain patterns (provide extensible pattern definition system)
- Performance impact on large codebases (efficient AST traversal and caching)

### Pattern Detection Accuracy
- Prioritize precision over recall to avoid false adjustments
- Use conservative confidence thresholds by default
- Provide detailed reasoning for all adjustments
- Allow manual override through configuration or inline comments

## Usage Examples

### Basic Context-Aware Analysis
```bash
# Enable context-aware complexity scoring
debtmap analyze . --context-aware

# Show detailed pattern detection information
debtmap analyze . --context-aware --detailed

# Configure custom adjustment factors
debtmap analyze . --context-aware --config custom-patterns.toml
```

### Configuration Examples
```toml
# custom-patterns.toml
[context_aware]
enabled = true

[context_aware.pattern_detection]
state_machine_min_states = 4
parser_min_functions = 3
algorithm_detection_enabled = true

[context_aware.adjustment_factors]
state_machine_factor = [0.7, 0.8]    # Less aggressive reduction
parser_factor = [0.8, 0.9]           # Minimal reduction
algorithm_factor = [0.4, 0.6]        # More aggressive reduction

[context_aware.confidence_thresholds]
min_pattern_confidence = 0.8          # Higher confidence required
uncertainty_fallback = true           # Fall back to standard scoring
```

### JSON Output Example
```json
{
  "analysis": {
    "context_aware": {
      "enabled": true,
      "detected_patterns": [
        {
          "pattern_type": "StateMachine",
          "confidence": 0.87,
          "details": {
            "state_count": 6,
            "transition_complexity": 3.2
          },
          "adjustment_factor": 0.72
        }
      ],
      "complexity_adjustment": {
        "original_cyclomatic": 15,
        "adjusted_cyclomatic": 10.8,
        "original_cognitive": 18,
        "adjusted_cognitive": 12.96,
        "reasoning": "Applied state machine factor based on 87% confidence"
      }
    },
    "recommendations": {
      "pattern_specific": [
        {
          "pattern": "StateMachine",
          "priority": "High",
          "title": "Add state transition testing",
          "description": "Complex state transitions require comprehensive testing"
        }
      ]
    }
  }
}
```

## Expected Impact

After implementation:

1. **Reduced False Positives**: Well-designed but necessarily complex code (state machines, parsers, algorithms) will receive appropriate complexity scores
2. **Enhanced Accuracy**: Risk scores will better reflect actual technical debt rather than domain complexity
3. **Better Recommendations**: Pattern-specific guidance that acknowledges legitimate complexity while still identifying improvement opportunities
4. **Improved Developer Experience**: Developers won't see legitimate design patterns flagged as high-priority technical debt
5. **Maintained Sensitivity**: Accidental complexity and anti-patterns will receive increased attention and priority

This feature addresses a fundamental limitation in current complexity analysis tools by providing context-aware scoring that distinguishes between necessary and accidental complexity, leading to more accurate technical debt assessment and prioritization.

## Migration and Compatibility

- **Breaking Changes**: None - context-aware scoring is opt-in via CLI flag
- **Configuration Migration**: New configuration section with sensible defaults
- **Output Compatibility**: Enhanced output maintains existing structure with additional context information  
- **API Stability**: New functionality accessed through existing analysis pipeline with optional context-aware enhancement

Context-aware complexity scoring provides a significant improvement in analysis accuracy while maintaining full backward compatibility with existing workflows and tools.