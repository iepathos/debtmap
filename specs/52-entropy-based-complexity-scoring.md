---
number: 52
title: Entropy-Based Complexity Scoring
category: optimization
priority: high
status: draft
dependencies: [27, 46]
created: 2025-01-20
---

# Specification 52: Entropy-Based Complexity Scoring

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [27 (Context-Aware Complexity Scoring), 46 (Intelligent Pattern Learning System)]

## Context

Current complexity analysis in debtmap relies on traditional metrics like cyclomatic and cognitive complexity, which often produce false positives for legitimate patterns such as:
- Pattern matching functions with many similar branches
- Data validation functions with multiple checks
- Dispatcher functions routing to different handlers
- Configuration parsers with many options

These functions appear complex by traditional metrics but are actually simple, repetitive patterns that are easy to understand and maintain. Information theory provides a better approach: measuring the entropy (randomness/variety) of code patterns to distinguish between truly complex logic and repetitive structural patterns.

## Objective

Implement an entropy-based complexity scoring system that uses information theory to measure true code complexity, reducing false positives by 70-80% for pattern-based code while maintaining sensitivity to genuinely complex business logic.

## Requirements

### Functional Requirements

1. **Token Entropy Calculation**
   - Extract tokens from function AST (operators, keywords, identifiers, literals)
   - Calculate Shannon entropy of token distribution
   - Normalize entropy score to 0.0-1.0 range

2. **Pattern Similarity Detection**
   - Identify repeated AST subtree patterns within functions
   - Calculate similarity scores between branches in conditional statements
   - Detect common patterns (validation, dispatching, transformation)

3. **Hybrid Complexity Scoring**
   - Combine traditional metrics with entropy score
   - Apply entropy as a dampening factor for high-cyclomatic functions
   - Maintain backward compatibility with existing scoring

4. **Language Support**
   - Implement for Rust initially
   - Extend to JavaScript/TypeScript
   - Design language-agnostic abstraction layer

5. **Configurable Thresholds**
   - Allow entropy weight configuration in .debtmap.toml
   - Support per-language entropy normalization
   - Enable feature flag for gradual rollout

### Non-Functional Requirements

1. **Performance**
   - Entropy calculation must add <10% to analysis time
   - Cache entropy scores for unchanged functions
   - Use efficient token hashing algorithms

2. **Accuracy**
   - Reduce false positive rate by >70% for pattern-based code
   - Maintain >95% detection rate for genuine complexity
   - Provide explainable scoring rationale

3. **Maintainability**
   - Clean separation from existing complexity modules
   - Comprehensive unit tests with known patterns
   - Documentation of entropy calculation methodology

## Acceptance Criteria

- [ ] Token entropy calculator implemented with Shannon entropy formula
- [ ] AST pattern similarity detector identifies repeated structures
- [ ] Entropy score integrated into UnifiedScore calculation
- [ ] False positive rate reduced by >70% on test corpus of pattern functions
- [ ] Performance impact <10% on large codebases (10k+ functions)
- [ ] Configuration options added to .debtmap.toml
- [ ] Unit tests achieve >90% coverage of entropy module
- [ ] Integration tests validate false positive reduction
- [ ] Documentation explains entropy scoring methodology
- [ ] Backward compatibility maintained with existing scores

## Technical Details

### Implementation Approach

```rust
// New module: src/complexity/entropy.rs
pub struct EntropyAnalyzer {
    token_cache: HashMap<FunctionId, EntropyScore>,
}

pub struct EntropyScore {
    pub token_entropy: f64,        // 0.0-1.0, higher = more complex
    pub pattern_repetition: f64,   // 0.0-1.0, higher = more repetitive
    pub branch_similarity: f64,    // 0.0-1.0, higher = similar branches
    pub effective_complexity: f64, // Adjusted complexity score
}

impl EntropyAnalyzer {
    pub fn calculate_entropy(&self, ast: &syn::Item) -> EntropyScore {
        let tokens = self.extract_tokens(ast);
        let entropy = self.shannon_entropy(&tokens);
        let patterns = self.detect_patterns(ast);
        let similarity = self.branch_similarity(ast);
        
        EntropyScore {
            token_entropy: entropy,
            pattern_repetition: patterns,
            branch_similarity: similarity,
            effective_complexity: self.adjust_complexity(entropy, patterns),
        }
    }
    
    fn shannon_entropy(&self, tokens: &[Token]) -> f64 {
        let frequencies = self.count_frequencies(tokens);
        let total = tokens.len() as f64;
        
        -frequencies.values()
            .map(|&count| {
                let p = count as f64 / total;
                p * p.log2()
            })
            .sum::<f64>() / total.log2() // Normalize to 0-1
    }
}
```

### Architecture Changes

1. **New Module Structure**
   ```
   src/complexity/
   ├── mod.rs           # Existing complexity module
   ├── entropy.rs       # New entropy analysis
   ├── patterns.rs      # Existing pattern detection
   └── integration.rs   # Integration with unified scoring
   ```

2. **Modified UnifiedScore Calculation**
   ```rust
   // In src/priority/unified_scorer.rs
   pub fn calculate_unified_score_with_entropy(
       metrics: &FunctionMetrics,
       entropy: &EntropyScore,
       context: &AnalysisContext,
   ) -> UnifiedScore {
       let base_complexity = metrics.cyclomatic as f64;
       
       // Apply entropy dampening for pattern-based code
       let adjusted_complexity = if entropy.pattern_repetition > 0.7 {
           base_complexity * 0.3  // High repetition = low risk
       } else if entropy.token_entropy < 0.4 {
           base_complexity * 0.5  // Low entropy = simple patterns
       } else {
           base_complexity       // High entropy = genuine complexity
       };
       
       // Continue with existing scoring...
   }
   ```

### Data Structures

```rust
// Token representation for entropy calculation
#[derive(Hash, Eq, PartialEq, Clone)]
enum TokenType {
    Keyword(String),      // if, match, for, etc.
    Operator(String),     // +, -, ==, etc.
    Identifier(String),   // Variable/function names
    Literal(LiteralType), // Numbers, strings, etc.
    Punctuation(char),    // {, }, (, ), etc.
}

// Pattern detection results
struct PatternAnalysis {
    repeated_structures: Vec<AstPattern>,
    branch_groups: Vec<BranchGroup>,
    similarity_matrix: Vec<Vec<f64>>,
}

// Configuration structure
#[derive(Deserialize)]
struct EntropyConfig {
    enabled: bool,
    weight: f64,              // 0.0-1.0, weight in final score
    min_tokens: usize,        // Minimum tokens for entropy calc
    pattern_threshold: f64,   // Similarity threshold for patterns
}
```

### APIs and Interfaces

```rust
// Public API in src/complexity/entropy.rs
pub trait EntropyCalculator {
    fn calculate_entropy(&self, function: &FunctionAst) -> EntropyScore;
    fn explain_score(&self, score: &EntropyScore) -> String;
}

// Integration point in src/analyzers/mod.rs
pub trait ComplexityAnalyzer {
    fn analyze_complexity(&self, function: &Function) -> ComplexityResult;
    fn analyze_with_entropy(&self, function: &Function) -> EntropyAwareResult;
}
```

## Dependencies

- **Prerequisites**: 
  - Spec 27 (Context-Aware Complexity Scoring) - Provides complexity framework
  - Spec 46 (Intelligent Pattern Learning) - Provides pattern detection base

- **Affected Components**:
  - `src/complexity/mod.rs` - Integration point
  - `src/priority/unified_scorer.rs` - Score calculation updates
  - `src/analyzers/*.rs` - Language-specific implementations

- **External Dependencies**:
  - No new external crates required
  - Uses existing syn and tree-sitter parsers

## Testing Strategy

- **Unit Tests**:
  - Test entropy calculation with known token distributions
  - Verify pattern detection on synthetic examples
  - Test score adjustments for various entropy levels

- **Integration Tests**:
  - Test false positive reduction on pattern-heavy code
  - Verify genuine complexity still detected
  - Test across all supported languages

- **Performance Tests**:
  - Benchmark entropy calculation on large functions
  - Measure memory usage of token caching
  - Profile overall analysis time impact

- **User Acceptance**:
  - Run on known false positive examples from FALSE_POSITIVE_ANALYSIS.md
  - Compare before/after scores on real codebases
  - Gather feedback on score accuracy improvements

## Documentation Requirements

- **Code Documentation**:
  - Document entropy calculation algorithm
  - Explain token extraction process
  - Provide examples of pattern detection

- **User Documentation**:
  - Add entropy scoring explanation to README
  - Document configuration options in .debtmap.toml
  - Provide tuning guidelines for different codebases

- **Architecture Updates**:
  - Update ARCHITECTURE.md with entropy module
  - Document data flow through scoring pipeline
  - Add entropy concepts to complexity documentation

## Implementation Notes

### Known Patterns to Handle

1. **Switch/Match Statements**: High cyclomatic but low entropy when cases are similar
2. **Validation Functions**: Multiple checks but repetitive structure
3. **Factory Functions**: Many branches but predictable patterns
4. **Configuration Parsers**: Long but straightforward option handling

### Entropy Calculation Details

Shannon entropy formula: H(X) = -Σ(p(x) * log₂(p(x)))
- Normalize by log₂(n) where n = total tokens
- Range: 0.0 (all tokens identical) to 1.0 (maximum variety)

### Pattern Similarity Algorithm

1. Convert AST branches to normalized token sequences
2. Calculate Levenshtein distance between sequences
3. Group branches with >80% similarity
4. Higher similarity = lower effective complexity

## Migration and Compatibility

### Backward Compatibility
- Entropy scoring is opt-in via configuration
- Existing scores remain available
- Can run both scoring methods in parallel

### Migration Path
1. Phase 1: Deploy with entropy disabled by default
2. Phase 2: Enable for specific file patterns
3. Phase 3: Gradual rollout with monitoring
4. Phase 4: Make default with opt-out option

### Configuration Migration
```toml
# .debtmap.toml
[complexity.entropy]
enabled = true
weight = 0.5  # 50% weight in final score
min_tokens = 20
pattern_threshold = 0.8

# Per-language overrides
[complexity.entropy.rust]
weight = 0.6  # Higher weight for Rust

[complexity.entropy.javascript]
weight = 0.4  # Lower weight for JS
```

### Breaking Changes
- None - fully backward compatible
- New scores are additive, not replacing existing ones

## Success Metrics

1. **False Positive Reduction**: >70% reduction in pattern-based false positives
2. **Detection Accuracy**: Maintain >95% detection of genuine complexity
3. **Performance Impact**: <10% increase in analysis time
4. **User Satisfaction**: Positive feedback on reduced noise
5. **Adoption Rate**: >80% of users enable entropy scoring within 3 months