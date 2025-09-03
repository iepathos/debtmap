---
number: 90
title: Language-Agnostic Entropy Framework
category: foundation
priority: high
status: draft
dependencies: []
created: 2025-09-03
---

# Specification 90: Language-Agnostic Entropy Framework

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current entropy analysis implementation is duplicated across language analyzers with significant code repetition. The Rust analyzer has a complete entropy implementation in `src/complexity/entropy.rs`, JavaScript has its own implementation in `src/analyzers/javascript/entropy.rs`, and Python lacks entropy analysis entirely (spec 70). Each implementation duplicates core algorithms like Shannon entropy calculation, pattern detection, and complexity adjustment.

This duplication violates DRY principles and makes it difficult to:
- Maintain consistent entropy scoring across languages
- Add entropy support to new languages
- Update entropy algorithms uniformly
- Share improvements across analyzers

## Objective

Create a language-agnostic entropy framework that separates universal entropy calculation algorithms from language-specific token extraction and pattern detection, enabling consistent entropy analysis across all supported languages while eliminating code duplication.

## Requirements

### Functional Requirements
- Extract core entropy algorithms into a shared module
- Define language-agnostic interfaces for entropy analysis
- Support pluggable language-specific implementations
- Maintain backward compatibility with existing entropy scores
- Enable easy addition of new language analyzers

### Non-Functional Requirements
- Zero performance regression from refactoring
- Maintain thread-safety for parallel analysis
- Keep memory footprint equivalent to current implementation
- Preserve deterministic entropy calculations

## Acceptance Criteria

- [ ] Core entropy module created with language-agnostic algorithms
- [ ] LanguageEntropyAnalyzer trait defined for language-specific parts
- [ ] Shannon entropy calculation extracted to shared module
- [ ] Pattern repetition detection generalized
- [ ] Branch similarity calculation abstracted
- [ ] Rust analyzer migrated to new framework
- [ ] JavaScript analyzer migrated to new framework
- [ ] All existing entropy tests pass without modification
- [ ] Performance benchmarks show no regression
- [ ] Documentation updated with new architecture

## Technical Details

### Implementation Approach

1. **Create Core Entropy Module** (`src/complexity/entropy_core.rs`)
   - Move Shannon entropy calculation
   - Extract complexity adjustment logic
   - Generalize pattern counting algorithms
   - Abstract branch similarity metrics

2. **Define Language Trait** (`src/complexity/entropy_traits.rs`)
   ```rust
   pub trait LanguageEntropyAnalyzer: Send + Sync {
       type AstNode;
       type Token: EntropyToken;
       
       fn extract_tokens(&self, node: &Self::AstNode) -> Vec<Self::Token>;
       fn detect_patterns(&self, node: &Self::AstNode) -> PatternMetrics;
       fn calculate_branch_similarity(&self, node: &Self::AstNode) -> f64;
       fn analyze_structure(&self, node: &Self::AstNode) -> (usize, u32);
   }
   
   pub trait EntropyToken: Clone + Hash + Eq {
       fn to_category(&self) -> TokenCategory;
       fn weight(&self) -> f64;
   }
   ```

3. **Refactor Existing Analyzers**
   - Implement trait for RustEntropyAnalyzer
   - Implement trait for JavaScriptEntropyAnalyzer
   - Preserve all existing functionality

### Architecture Changes

```
src/complexity/
├── entropy.rs              # Existing Rust-specific (to be refactored)
├── entropy_core.rs         # NEW: Language-agnostic algorithms
├── entropy_traits.rs       # NEW: Trait definitions
├── token_classifier.rs     # Existing token classification
└── languages/              # NEW: Language-specific implementations
    ├── rust.rs            # Rust-specific entropy
    └── javascript.rs      # JavaScript-specific entropy
```

### Data Structures

```rust
// Language-agnostic entropy score (unchanged)
pub struct EntropyScore {
    pub token_entropy: f64,
    pub pattern_repetition: f64,
    pub branch_similarity: f64,
    pub effective_complexity: f64,
    pub unique_variables: usize,
    pub max_nesting: u32,
    pub dampening_applied: f64,
}

// Generic token category
pub enum TokenCategory {
    Keyword,
    Operator,
    Identifier,
    Literal,
    ControlFlow,
    FunctionCall,
    Custom(String),
}

// Pattern metrics
pub struct PatternMetrics {
    pub total_patterns: usize,
    pub unique_patterns: usize,
    pub repetition_ratio: f64,
}
```

### APIs and Interfaces

```rust
// Universal entropy calculator
pub struct UniversalEntropyCalculator {
    cache: HashMap<String, EntropyScore>,
    config: EntropyConfig,
}

impl UniversalEntropyCalculator {
    pub fn calculate<L: LanguageEntropyAnalyzer>(
        &mut self,
        analyzer: &L,
        node: &L::AstNode,
    ) -> EntropyScore;
    
    pub fn shannon_entropy<T: EntropyToken>(&self, tokens: &[T]) -> f64;
    pub fn adjust_complexity(&self, entropy: f64, patterns: f64, similarity: f64) -> f64;
    pub fn apply_dampening(&self, score: &EntropyScore) -> f64;
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: 
  - `src/complexity/entropy.rs`
  - `src/analyzers/rust.rs`
  - `src/analyzers/javascript/entropy.rs`
  - `src/analyzers/javascript/complexity.rs`
- **External Dependencies**: None (uses existing crates)

## Testing Strategy

- **Unit Tests**: Test each component independently
  - Core algorithms with synthetic data
  - Language analyzers with sample code
  - Cache functionality
- **Integration Tests**: Verify language analyzers work with framework
- **Regression Tests**: Ensure entropy scores remain consistent
- **Performance Tests**: Benchmark against current implementation

## Documentation Requirements

- **Code Documentation**: 
  - Document trait requirements clearly
  - Provide implementation examples
  - Explain token categorization
- **Architecture Updates**: Update ARCHITECTURE.md with new module structure
- **Migration Guide**: Document how to migrate existing analyzers

## Implementation Notes

- Start with extracting algorithms, keeping interfaces minimal
- Migrate one analyzer at a time to validate approach
- Use generics judiciously to avoid compilation overhead
- Consider using dynamic dispatch for plugin architecture later
- Maintain compatibility with existing EntropyScore structure

## Migration and Compatibility

During prototype phase: This refactoring should not change external APIs or entropy scores. All existing functionality must be preserved while improving internal structure. The migration should be transparent to users of the entropy analysis.