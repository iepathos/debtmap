---
number: 191
title: Semantic Module Naming
category: optimization
priority: high
status: draft
dependencies: [188, 190]
created: 2025-11-20
---

# Specification 191: Semantic Module Naming

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: [188 - Intelligent Module Split Recommendations, 190 - Minimum Split Size Enforcement]

## Context

Current module split recommendations frequently use generic, non-descriptive names that fail to communicate the module's purpose and domain:

### Problem Examples from Latest Analysis

**Issue 1: "Unknown" Modules**
```
RECOMMENDED SPLITS (3 modules):
  - god_object_detector/unknown.rs
    Category: Manage Unknown data and its transformations
    Size: 3 methods, ~45 lines

  - god_object_detector/unknown.rs   // DUPLICATE!
    Category: Manage Unknown data and its transformations
    Size: 13 methods, ~195 lines
```

**Issue 2: "Self" Modules**
```
  - god_object_analysis/self.rs
    Category: Manage Self data and its transformations
    Size: 8 methods, ~120 lines
    Methods: new(), default(), default(), eq(), default()...
```

**Issue 3: Generic Type-Based Names**
```
  - formatter/unifieddebtitem.rs
    Category: Manage UnifiedDebtItem data and its transformations
```

### Core Problems

1. **No Semantic Meaning**: "unknown.rs" provides zero information about module purpose
2. **Filename Conflicts**: Multiple splits named "unknown.rs" would collide
3. **Non-Descriptive Categories**: "Manage Unknown data and its transformations" is meaningless
4. **Implementation-Focused**: Names describe data types, not domain concepts or behaviors
5. **Poor Discoverability**: Developers can't find relevant code from generic names

### Current Naming Logic

```rust
// Simplified current approach
fn generate_module_name(category: &str) -> String {
    match category {
        s if s.contains("TypeAnalysis") => "typeanalysis.rs",
        s if s.contains("Self") => "self.rs",
        _ => "unknown.rs",  // Fallback for uncategorized
    }
}
```

Issues:
- Overly simplistic pattern matching
- No semantic analysis of method names or purposes
- No domain knowledge extraction
- Generic fallbacks instead of meaningful names

### Impact

From evaluation of debtmap's own output:
- 35% of recommended splits use "unknown.rs" naming
- 12% use generic type names (e.g., "unifieddebtitem.rs")
- 8% have filename collisions (multiple modules with same name)
- Recommendations appear low-quality and unimplementable to users

## Objective

Implement intelligent module naming that extracts semantic meaning from code structure, method names, and domain concepts to generate descriptive, unique, and actionable module names.

## Requirements

### Functional Requirements

1. **Semantic Name Extraction**
   - Analyze method names to identify common domain terms
   - Use verb-noun patterns to extract purposes (e.g., "format_item" → "formatting")
   - Identify domain-specific terminology from comments and documentation
   - Recognize standard software patterns (builders, validators, parsers, formatters)

2. **Multi-Strategy Naming**
   - **Strategy 1: Domain Terms** - Extract from method/type names (e.g., "coverage", "metrics", "validation")
   - **Strategy 2: Behavioral Patterns** - Identify by actions (e.g., "parsing", "serialization", "computation")
   - **Strategy 3: Architectural Layers** - Recognize by layer (e.g., "io", "core", "adapters")
   - **Strategy 4: Responsibility Analysis** - Use existing responsibility clustering data
   - **Strategy 5: Call Graph Analysis** - Identify modules by external interface patterns

3. **Name Quality Validation**
   - Reject generic names: "unknown", "self", "misc", "utils", "common"
   - Require minimum specificity score (>0.5)
   - Ensure uniqueness within same parent directory
   - Validate names are valid Rust module identifiers

4. **Confidence Scoring**
   - Assign confidence score to each generated name (0.0-1.0)
   - High confidence (>0.7): Use directly
   - Medium confidence (0.4-0.7): Show with qualifier "[suggested]"
   - Low confidence (<0.4): Use "needs_review_{domain}" pattern
   - Always prefer low-confidence descriptive names over "unknown"

5. **Multiple Name Candidates**
   - Generate 2-3 alternative names per split
   - Rank by confidence and specificity
   - Display top choice with alternatives in output
   - Allow users to see reasoning for each candidate

6. **Fallback Strategy**
   - When semantic extraction fails, use descriptive placeholders:
     - "needs_review_group_{N}" instead of "unknown"
     - Include top 3 method names in comment: "// Contains: method1, method2, method3"
   - Never output "unknown.rs" or "self.rs"

### Non-Functional Requirements

- **Accuracy**: >85% of names are human-approved in validation study
- **Uniqueness**: 100% of names are unique within parent directory
- **Performance**: Name generation adds <10% to analysis time
- **Determinism**: Same code produces same names across runs

## Acceptance Criteria

- [ ] No splits are named "unknown.rs", "self.rs", or "misc.rs"
- [ ] No filename collisions within same parent directory
- [ ] >85% of generated names score >0.5 on specificity metric
- [ ] Each split includes confidence score and 2-3 alternative names
- [ ] Names reflect domain concepts, not just data type names
- [ ] Regression test: debtmap's god_object_detector.rs splits have meaningful names
- [ ] Performance: Name generation adds <10% to analysis time
- [ ] Documentation: Output explains naming rationale for each split
- [ ] Fallback naming: Uses "needs_review_{domain}" with method hints, never "unknown"

## Technical Details

### Implementation Approach

**Phase 1: Semantic Analysis Pipeline**

```rust
pub struct SemanticNameGenerator {
    domain_extractor: DomainTermExtractor,
    pattern_recognizer: PatternRecognizer,
    layer_analyzer: LayerAnalyzer,
    specificity_scorer: SpecificityScorer,
}

impl SemanticNameGenerator {
    pub fn generate_names(&self, split: &ModuleSplit) -> Vec<NameCandidate> {
        let mut candidates = Vec::new();

        // Strategy 1: Domain terms from method names
        if let Some(domain_name) = self.extract_domain_name(split) {
            candidates.push(domain_name);
        }

        // Strategy 2: Behavioral patterns
        if let Some(behavior_name) = self.extract_behavioral_name(split) {
            candidates.push(behavior_name);
        }

        // Strategy 3: Architectural layer
        if let Some(layer_name) = self.extract_layer_name(split) {
            candidates.push(layer_name);
        }

        // Strategy 4: Responsibility clustering
        if let Some(responsibility_name) = self.extract_responsibility_name(split) {
            candidates.push(responsibility_name);
        }

        // Filter and rank
        candidates
            .into_iter()
            .filter(|c| self.is_valid_name(c))
            .filter(|c| c.specificity_score > 0.4)
            .sorted_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap())
            .take(3)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct NameCandidate {
    pub module_name: String,
    pub confidence: f64,
    pub specificity_score: f64,
    pub reasoning: String,
    pub strategy: NamingStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum NamingStrategy {
    DomainTerms,
    BehavioralPattern,
    ArchitecturalLayer,
    ResponsibilityCluster,
    CallGraphInterface,
}
```

**Phase 2: Domain Term Extraction**

```rust
pub struct DomainTermExtractor {
    common_verbs: HashSet<String>,
    domain_nouns: HashMap<String, Vec<String>>,
}

impl DomainTermExtractor {
    pub fn extract_domain_terms(&self, methods: &[String]) -> Vec<(String, f64)> {
        // Tokenize method names
        let tokens = methods
            .iter()
            .flat_map(|m| self.tokenize_method_name(m))
            .collect::<Vec<_>>();

        // Count term frequencies
        let term_freq = self.calculate_term_frequencies(&tokens);

        // Extract significant terms (appear in >30% of methods)
        term_freq
            .into_iter()
            .filter(|(_, freq)| *freq > 0.3)
            .filter(|(term, _)| !self.is_stop_word(term))
            .collect()
    }

    fn tokenize_method_name(&self, method: &str) -> Vec<String> {
        // Handle various naming conventions:
        // - snake_case: format_coverage_status → [format, coverage, status]
        // - camelCase: calculateMetrics → [calculate, metrics]
        // - Combined: get_APIKey → [get, api, key]

        let mut tokens = Vec::new();

        // Split on underscores
        for part in method.split('_') {
            // Split camelCase within each part
            tokens.extend(self.split_camel_case(part));
        }

        tokens
            .into_iter()
            .map(|s| s.to_lowercase())
            .filter(|s| s.len() > 2) // Remove very short tokens
            .collect()
    }

    fn split_camel_case(&self, s: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();

        for ch in s.chars() {
            if ch.is_uppercase() && !current.is_empty() {
                result.push(current.clone());
                current.clear();
            }
            current.push(ch);
        }

        if !current.is_empty() {
            result.push(current);
        }

        result
    }
}

impl DomainTermExtractor {
    pub fn generate_domain_name(&self, split: &ModuleSplit) -> Option<NameCandidate> {
        let terms = self.extract_domain_terms(&split.methods);

        if terms.is_empty() {
            return None;
        }

        // Find most significant term combination
        let (primary_term, primary_freq) = terms[0].clone();

        // Look for verb-noun pairs
        if let Some((verb, noun)) = self.find_verb_noun_pair(&terms) {
            return Some(NameCandidate {
                module_name: format!("{}_{}", verb, noun), // e.g., "format_coverage"
                confidence: 0.8,
                specificity_score: self.calculate_specificity(&format!("{}_{}", verb, noun)),
                reasoning: format!(
                    "Identified verb-noun pattern: '{}' + '{}' (frequency: {:.1}%)",
                    verb, noun, primary_freq * 100.0
                ),
                strategy: NamingStrategy::DomainTerms,
            });
        }

        // Single dominant term
        if primary_freq > 0.5 {
            Some(NameCandidate {
                module_name: format!("{}", primary_term),
                confidence: 0.7,
                specificity_score: self.calculate_specificity(&primary_term),
                reasoning: format!(
                    "Dominant term '{}' appears in {:.1}% of methods",
                    primary_term, primary_freq * 100.0
                ),
                strategy: NamingStrategy::DomainTerms,
            })
        } else {
            None
        }
    }
}
```

**Phase 3: Behavioral Pattern Recognition**

```rust
pub struct PatternRecognizer {
    patterns: Vec<BehaviorPattern>,
}

#[derive(Debug)]
struct BehaviorPattern {
    name: String,
    verbs: Vec<String>,
    confidence_threshold: f64,
}

impl PatternRecognizer {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                BehaviorPattern {
                    name: "formatting".into(),
                    verbs: vec!["format", "display", "render", "print", "show"].into_iter().map(String::from).collect(),
                    confidence_threshold: 0.6,
                },
                BehaviorPattern {
                    name: "validation".into(),
                    verbs: vec!["validate", "verify", "check", "ensure", "assert"].into_iter().map(String::from).collect(),
                    confidence_threshold: 0.6,
                },
                BehaviorPattern {
                    name: "parsing".into(),
                    verbs: vec!["parse", "extract", "read", "decode", "interpret"].into_iter().map(String::from).collect(),
                    confidence_threshold: 0.6,
                },
                BehaviorPattern {
                    name: "computation".into(),
                    verbs: vec!["calculate", "compute", "evaluate", "measure", "analyze"].into_iter().map(String::from).collect(),
                    confidence_threshold: 0.6,
                },
                BehaviorPattern {
                    name: "transformation".into(),
                    verbs: vec!["convert", "transform", "map", "translate", "adapt"].into_iter().map(String::from).collect(),
                    confidence_threshold: 0.6,
                },
                BehaviorPattern {
                    name: "serialization".into(),
                    verbs: vec!["serialize", "deserialize", "encode", "decode", "marshal"].into_iter().map(String::from).collect(),
                    confidence_threshold: 0.7,
                },
            ],
        }
    }

    pub fn recognize_pattern(&self, split: &ModuleSplit) -> Option<NameCandidate> {
        for pattern in &self.patterns {
            let match_score = self.calculate_pattern_match(split, pattern);

            if match_score > pattern.confidence_threshold {
                return Some(NameCandidate {
                    module_name: pattern.name.clone(),
                    confidence: match_score,
                    specificity_score: 0.7, // Patterns are moderately specific
                    reasoning: format!(
                        "Recognized {} pattern ({:.1}% of methods match)",
                        pattern.name, match_score * 100.0
                    ),
                    strategy: NamingStrategy::BehavioralPattern,
                });
            }
        }

        None
    }

    fn calculate_pattern_match(&self, split: &ModuleSplit, pattern: &BehaviorPattern) -> f64 {
        let matching_methods = split
            .methods
            .iter()
            .filter(|method| {
                let method_lower = method.to_lowercase();
                pattern.verbs.iter().any(|verb| method_lower.contains(verb))
            })
            .count();

        matching_methods as f64 / split.methods.len() as f64
    }
}
```

**Phase 4: Specificity Scoring**

```rust
pub struct SpecificityScorer {
    generic_terms: HashSet<String>,
    domain_specific_terms: HashMap<String, f64>,
}

impl SpecificityScorer {
    pub fn new() -> Self {
        Self {
            generic_terms: [
                "unknown", "self", "misc", "utils", "common", "helpers",
                "data", "types", "structs", "impl", "methods", "functions",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),

            domain_specific_terms: [
                ("coverage", 0.9),
                ("metrics", 0.85),
                ("parsing", 0.9),
                ("formatting", 0.85),
                ("validation", 0.85),
                ("complexity", 0.9),
                ("analysis", 0.8),
                ("optimization", 0.9),
            ]
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect(),
        }
    }

    pub fn calculate_specificity(&self, name: &str) -> f64 {
        // Check against generic terms (disqualifies)
        if self.generic_terms.contains(&name.to_lowercase()) {
            return 0.0;
        }

        // Check against domain-specific terms (high score)
        if let Some(score) = self.domain_specific_terms.get(&name.to_lowercase()) {
            return *score;
        }

        // Calculate based on characteristics
        let mut score = 0.5; // Base score

        // Longer names are typically more specific
        if name.len() > 8 {
            score += 0.1;
        }

        // Contains domain terms
        if name.contains('_') {
            score += 0.1; // Compound names are more specific
        }

        // Contains specific verbs
        let specific_verbs = ["format", "parse", "validate", "calculate", "analyze"];
        if specific_verbs.iter().any(|v| name.contains(v)) {
            score += 0.15;
        }

        score.min(1.0)
    }
}
```

**Phase 5: Uniqueness Validation**

```rust
pub struct NameUniquenessValidator {
    used_names: HashMap<PathBuf, HashSet<String>>,
}

impl NameUniquenessValidator {
    pub fn ensure_unique_name(
        &mut self,
        parent_path: &Path,
        candidates: Vec<NameCandidate>,
    ) -> NameCandidate {
        let used = self.used_names.entry(parent_path.to_path_buf()).or_default();

        // Try each candidate in order of confidence
        for candidate in &candidates {
            if !used.contains(&candidate.module_name) {
                used.insert(candidate.module_name.clone());
                return candidate.clone();
            }
        }

        // All candidates are used, generate disambiguated name
        let base_name = &candidates[0].module_name;
        let mut counter = 2;

        loop {
            let disambiguated = format!("{}_{}", base_name, counter);
            if !used.contains(&disambiguated) {
                used.insert(disambiguated.clone());
                return NameCandidate {
                    module_name: disambiguated.clone(),
                    confidence: candidates[0].confidence * 0.8, // Lower confidence for disambiguated
                    specificity_score: candidates[0].specificity_score,
                    reasoning: format!("{} (disambiguated to avoid collision)", candidates[0].reasoning),
                    strategy: candidates[0].strategy,
                };
            }
            counter += 1;
        }
    }
}
```

### Architecture Changes

**New Module**: `src/organization/semantic_naming.rs`
```rust
pub mod semantic_naming {
    mod domain_extractor;
    mod pattern_recognizer;
    mod layer_analyzer;
    mod specificity_scorer;
    mod uniqueness_validator;

    pub use domain_extractor::DomainTermExtractor;
    pub use pattern_recognizer::PatternRecognizer;
    pub use layer_analyzer::LayerAnalyzer;
    pub use specificity_scorer::SpecificityScorer;
    pub use uniqueness_validator::NameUniquenessValidator;

    pub struct SemanticNameGenerator {
        // ... as shown above
    }
}
```

**Modified**: `src/organization/god_object_analysis.rs`
```rust
#[derive(Debug, Clone)]
pub struct ModuleSplit {
    pub module_name: String,
    pub alternative_names: Vec<NameCandidate>, // NEW
    pub naming_confidence: f64,                 // NEW
    pub naming_strategy: NamingStrategy,        // NEW
    // ... existing fields
}
```

### Output Format Changes

**Before**:
```
RECOMMENDED SPLITS (3 modules):
  - god_object_detector/unknown.rs
    Category: Manage Unknown data and its transformations
    Size: 13 methods, ~195 lines
```

**After**:
```
RECOMMENDED SPLITS (3 modules):
  - god_object_detector/complexity_analysis.rs [confidence: 0.82]
    Category: Complexity calculation and metrics
    Size: 13 methods, ~195 lines
    Naming: Identified from method patterns (calculate_complexity, analyze_complexity, ...)
    Alternatives: metrics_computation.rs [0.75], complexity_scoring.rs [0.68]
```

**Low Confidence Case**:
```
  - god_object_detector/needs_review_group_2.rs [confidence: 0.35]
    Category: Mixed responsibilities - requires manual review
    Size: 8 methods, ~120 lines
    Naming: Auto-generated fallback (low confidence)
    Contains: new(), default(), clone(), eq(), validate(), process(), transform(), finalize()
    Suggestion: Review methods and choose domain-appropriate name
```

## Dependencies

- **Prerequisites**:
  - [188] Intelligent Module Split Recommendations (base clustering)
  - [190] Minimum Split Size Enforcement (ensures viable splits)

- **Affected Components**:
  - `src/organization/god_object_detector.rs` - Use semantic naming
  - `src/organization/god_object_analysis.rs` - Update ModuleSplit structure
  - `src/priority/formatter.rs` - Display naming rationale

- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_term_extraction() {
        let methods = vec![
            "format_coverage_status".into(),
            "format_coverage_factor".into(),
            "calculate_coverage_percentage".into(),
        ];
        let split = create_test_split(methods);

        let extractor = DomainTermExtractor::new();
        let terms = extractor.extract_domain_terms(&split.methods);

        assert!(terms.iter().any(|(term, _)| term == "coverage"));
        assert!(terms.iter().any(|(term, _)| term == "format"));
    }

    #[test]
    fn test_rejects_generic_names() {
        let scorer = SpecificityScorer::new();

        assert_eq!(scorer.calculate_specificity("unknown"), 0.0);
        assert_eq!(scorer.calculate_specificity("self"), 0.0);
        assert_eq!(scorer.calculate_specificity("misc"), 0.0);
        assert_eq!(scorer.calculate_specificity("utils"), 0.0);
    }

    #[test]
    fn test_pattern_recognition() {
        let methods = vec![
            "format_item".into(),
            "format_details".into(),
            "format_summary".into(),
            "display_result".into(),
        ];
        let split = create_test_split(methods);

        let recognizer = PatternRecognizer::new();
        let pattern = recognizer.recognize_pattern(&split);

        assert!(pattern.is_some());
        let candidate = pattern.unwrap();
        assert_eq!(candidate.module_name, "formatting");
        assert!(candidate.confidence > 0.7);
    }

    #[test]
    fn test_uniqueness_enforcement() {
        let mut validator = NameUniquenessValidator::new();
        let parent = Path::new("src/organization");

        let candidates1 = vec![
            NameCandidate {
                module_name: "metrics".into(),
                confidence: 0.9,
                ..Default::default()
            },
        ];

        let candidates2 = vec![
            NameCandidate {
                module_name: "metrics".into(), // Collision!
                confidence: 0.85,
                ..Default::default()
            },
        ];

        let name1 = validator.ensure_unique_name(parent, candidates1);
        let name2 = validator.ensure_unique_name(parent, candidates2);

        assert_eq!(name1.module_name, "metrics");
        assert_eq!(name2.module_name, "metrics_2"); // Disambiguated
    }

    #[test]
    fn test_verb_noun_extraction() {
        let methods = vec![
            "calculate_metrics".into(),
            "compute_metrics".into(),
            "analyze_metrics".into(),
        ];
        let split = create_test_split(methods);

        let extractor = DomainTermExtractor::new();
        let name = extractor.generate_domain_name(&split);

        assert!(name.is_some());
        let candidate = name.unwrap();
        assert!(
            candidate.module_name.contains("metrics") ||
            candidate.module_name.contains("calculate")
        );
    }
}
```

### Integration Tests

```rust
#[test]
fn test_no_generic_names_in_output() {
    let detector = GodObjectDetector::with_semantic_naming();
    let ast = parse_file("tests/fixtures/large_formatter.rs");

    let analysis = detector.analyze_enhanced(Path::new("formatter.rs"), &ast);

    let generic_names = ["unknown", "self", "misc", "utils", "common"];

    for split in &analysis.recommended_splits {
        for generic in &generic_names {
            assert!(
                !split.module_name.contains(generic),
                "Found generic name '{}' in split: {}",
                generic,
                split.module_name
            );
        }
    }
}

#[test]
fn test_name_uniqueness_across_splits() {
    let detector = GodObjectDetector::new();
    let ast = parse_file("tests/fixtures/large_formatter.rs");

    let analysis = detector.analyze_enhanced(Path::new("formatter.rs"), &ast);
    let splits = analysis.recommended_splits;

    let names: HashSet<_> = splits.iter().map(|s| &s.module_name).collect();

    assert_eq!(
        names.len(),
        splits.len(),
        "Found duplicate module names: {:?}",
        splits.iter().map(|s| &s.module_name).collect::<Vec<_>>()
    );
}

#[test]
fn test_high_confidence_names() {
    let detector = GodObjectDetector::new();
    let ast = parse_file("tests/fixtures/large_formatter.rs");

    let analysis = detector.analyze_enhanced(Path::new("formatter.rs"), &ast);

    let high_confidence_count = analysis
        .recommended_splits
        .iter()
        .filter(|s| s.naming_confidence > 0.6)
        .count();

    let total_splits = analysis.recommended_splits.len();

    // At least 70% should have high confidence names
    assert!(
        high_confidence_count as f64 / total_splits as f64 > 0.7,
        "Only {}/{} splits have high-confidence names",
        high_confidence_count,
        total_splits
    );
}
```

### Regression Tests

```rust
#[test]
fn test_debtmap_self_analysis_naming() {
    let output = run_debtmap_on_itself();
    let splits = parse_splits_from_output(&output, "god_object_detector.rs");

    // No generic names
    for split in &splits {
        assert!(!split.name.contains("unknown"));
        assert!(!split.name.contains("self"));
        assert!(split.name != "misc");
    }

    // All names are unique
    let unique_names: HashSet<_> = splits.iter().map(|s| &s.name).collect();
    assert_eq!(unique_names.len(), splits.len());

    // Most names have good confidence
    let high_conf = splits.iter().filter(|s| s.confidence > 0.6).count();
    assert!(high_conf as f64 / splits.len() as f64 > 0.7);
}
```

## Documentation Requirements

### Code Documentation

- Document each naming strategy with examples
- Explain specificity scoring algorithm
- Document fallback naming strategy

### User Documentation

Add to README and user guide:

```markdown
## Module Split Naming

Debtmap uses semantic analysis to generate meaningful module names:

### Naming Strategies

1. **Domain Terms**: Extracts common terms from method names
   - Example: Methods like `format_coverage`, `calculate_coverage` → `coverage.rs`

2. **Behavioral Patterns**: Recognizes common behaviors
   - Example: Methods starting with `parse_`, `extract_` → `parsing.rs`

3. **Architectural Layers**: Identifies by layer
   - Example: Methods with I/O operations → `io.rs`

### Name Confidence

Each split includes a confidence score:
- **High (>0.7)**: Strong semantic signal, use directly
- **Medium (0.4-0.7)**: Reasonable guess, review recommended
- **Low (<0.4)**: Auto-generated, manual naming required

### Alternative Names

Every split includes 2-3 alternative name suggestions. Review alternatives if primary name doesn't fit your domain model.
```

### Architecture Updates

Update `ARCHITECTURE.md`:

```markdown
## Semantic Module Naming

Split recommendations use multi-strategy naming:

1. **Domain Term Extraction**: NLP-inspired tokenization of method names
2. **Pattern Recognition**: Matches against common software patterns
3. **Layer Analysis**: Identifies architectural layer from dependencies
4. **Specificity Scoring**: Ensures names are descriptive, not generic
5. **Uniqueness Validation**: Guarantees no filename collisions

Never outputs generic names like "unknown.rs" - uses descriptive fallbacks like "needs_review_group_N" with method hints.
```

## Implementation Notes

### Key Design Decisions

1. **Multiple Strategies**: Different codebases have different naming conventions
   - Some are verb-heavy (functional): `calculate_`, `process_`, `transform_`
   - Some are noun-heavy (OOP): `MetricsCalculator`, `DataProcessor`
   - Multi-strategy approach adapts to codebase style

2. **Confidence Scoring**: Honest about uncertainty
   - Better to say "low confidence, needs review" than to hide uncertainty
   - Users can make informed decisions about accepting recommendations

3. **Fallback Naming**: Never use "unknown"
   - "needs_review_group_N" is more actionable
   - Including method hints guides manual naming

4. **Performance**: Name generation is fast
   - Primarily string operations and pattern matching
   - No expensive NLP or ML models required
   - <10% overhead on total analysis time

### Potential Gotchas

1. **Domain-Specific Terms**: Generic system may miss project-specific terminology
   - **Mitigation**: Allow custom domain term dictionary via config file

2. **Abbreviations**: May not recognize project abbreviations (e.g., "FMT" for "formatter")
   - **Mitigation**: Expand common abbreviations in tokenization

3. **Over-Specific Names**: Very long, compound names
   - **Mitigation**: Limit name length to 3 words max, prefer primary term

## Migration and Compatibility

### Breaking Changes

- **None**: Only affects recommendation output format, not core analysis

### Backward Compatibility

- Old output format available via `--legacy-naming` flag
- JSON output includes both old and new names for comparison

## Success Metrics

- **Zero generic names**: 0% of splits named "unknown", "self", or "misc"
- **High specificity**: >85% of names score >0.5 on specificity
- **Uniqueness**: 100% of names unique within parent directory
- **User acceptance**: >80% of users find names appropriate in validation study
- **Confidence calibration**: When confidence >0.7, users agree with name >85% of time
