---
number: 210
title: Context-Aware God Object Recommendations
category: optimization
priority: medium
status: draft
dependencies: [206, 208]
created: 2025-12-15
---

# Specification 210: Context-Aware God Object Recommendations

**Category**: optimization
**Priority**: medium (P1)
**Status**: draft
**Dependencies**: Spec 206 (Cohesion Gate), Spec 208 (Domain-Aware Grouping)

## Context

Current God Object recommendations are generic and often unhelpful:
- "Extract 5 sub-orchestrators to reduce coordination complexity"
- The number comes from `responsibility_count.clamp(2, 5)` without considering actual structure
- Recommendations don't account for struct cohesion or domain alignment

### Current Problem

For a struct like `CrossModuleTracker`:
```
Current recommendation:
  "Extract 5 sub-orchestrators to reduce coordination complexity"

Issues:
1. The number "5" comes from method-prefix responsibility count, not domain analysis
2. "Sub-orchestrators" doesn't make sense for a cohesive tracker
3. No specific guidance on what to extract or why
4. Doesn't consider that the struct might actually be well-designed
```

### Desired Behavior

```
Context-aware recommendation:

For cohesive structs (high domain alignment):
  "Consider internal refactoring: extract the 44-line analyze_workspace()
   method into smaller phases. The struct's cohesion is good."

For true God Objects (low domain alignment):
  "Split by domain: extract ParsingModule (parse_json, parse_xml, parse_csv),
   RenderingModule (render_html, render_pdf), and ValidationModule
   (validate_email, validate_phone). These 3 domains have no shared state."

For borderline cases:
  "High method count but moderate cohesion. Consider:
   - Extract the 8 data-access methods into a DataAccessLayer
   - Keep the 4 business logic methods in the core struct"
```

## Objective

Generate context-aware, actionable recommendations that:
1. Consider struct cohesion and domain alignment
2. Identify specific methods or groups to extract
3. Explain the rationale based on actual analysis
4. Provide different advice for different scenarios

## Requirements

### Functional Requirements

1. **Scenario Detection**: Identify the type of God Object issue:
   - High cohesion + large size → Internal refactoring recommendation
   - Low cohesion + multiple domains → Domain-based split recommendation
   - Single domain + many methods → Layered extraction recommendation
   - Borderline case → Graduated recommendation with options

2. **Specific Recommendations**: Generate actionable advice:
   - Name specific methods/groups to extract
   - Suggest target module/struct names based on domain
   - Estimate complexity of recommended refactoring
   - Identify potential dependencies between extractions

3. **Rationale**: Explain why the recommendation is made:
   - Reference cohesion score
   - List domains detected
   - Cite specific metrics (method count, LOC, complexity)

### Non-Functional Requirements

- Recommendations must be deterministic
- Generated text must be grammatically correct
- Should not produce overly long recommendations (max ~200 words)

## Acceptance Criteria

- [ ] Cohesive structs get "internal refactoring" recommendations, not "extract sub-orchestrators"
- [ ] True God Objects get domain-specific split recommendations with method lists
- [ ] Recommendations include rationale based on cohesion score and domain analysis
- [ ] Long method extraction is suggested when single methods exceed 30 lines
- [ ] Generated module names are valid Rust identifiers
- [ ] Existing tests continue to pass
- [ ] New tests validate recommendation quality

## Technical Details

### Implementation Approach

#### 1. Recommendation Context

```rust
#[derive(Debug, Clone)]
pub struct RecommendationContext {
    pub cohesion_score: f64,
    pub domain_groups: HashMap<String, Vec<String>>,
    pub long_methods: Vec<LongMethodInfo>,
    pub total_methods: usize,
    pub substantive_methods: usize,  // Excluding accessors
    pub largest_method_loc: usize,
    pub struct_name: String,
}

#[derive(Debug, Clone)]
pub struct LongMethodInfo {
    pub name: String,
    pub line_count: usize,
    pub complexity: u32,
}
```

#### 2. Scenario Classification

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum GodObjectScenario {
    /// High cohesion, large size - suggest internal refactoring
    CohesiveLarge {
        long_methods: Vec<String>,
    },
    /// Low cohesion, multiple distinct domains - suggest domain splits
    MultiDomain {
        domains: Vec<DomainSplit>,
    },
    /// Single domain, too many methods - suggest layered extraction
    SingleDomainLarge {
        suggested_layers: Vec<LayerSplit>,
    },
    /// Borderline case - provide options
    Borderline {
        primary_recommendation: String,
        alternatives: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct DomainSplit {
    pub domain_name: String,
    pub suggested_module_name: String,
    pub methods: Vec<String>,
    pub estimated_loc: usize,
}

#[derive(Debug, Clone)]
pub struct LayerSplit {
    pub layer_name: String,
    pub description: String,
    pub methods: Vec<String>,
}

pub fn classify_scenario(context: &RecommendationContext) -> GodObjectScenario {
    let domain_count = context.domain_groups.len();
    let has_long_methods = !context.long_methods.is_empty();

    if context.cohesion_score > 0.5 {
        // High cohesion - suggest internal refactoring
        if has_long_methods {
            GodObjectScenario::CohesiveLarge {
                long_methods: context.long_methods.iter()
                    .map(|m| m.name.clone())
                    .collect(),
            }
        } else {
            GodObjectScenario::Borderline {
                primary_recommendation: format!(
                    "Struct has good cohesion ({:.0}%). Consider if size is justified.",
                    context.cohesion_score * 100.0
                ),
                alternatives: vec![
                    "Extract utility methods to a helper module".into(),
                    "Consider if all methods need to be public".into(),
                ],
            }
        }
    } else if domain_count >= 3 {
        // Multiple distinct domains - suggest domain splits
        let domains = context.domain_groups.iter()
            .map(|(name, methods)| DomainSplit {
                domain_name: name.clone(),
                suggested_module_name: generate_module_name(name),
                methods: methods.clone(),
                estimated_loc: methods.len() * 15, // Rough estimate
            })
            .collect();

        GodObjectScenario::MultiDomain { domains }
    } else if domain_count <= 2 && context.substantive_methods > 10 {
        // Single domain, many methods - suggest layers
        let layers = suggest_layer_splits(context);
        GodObjectScenario::SingleDomainLarge { suggested_layers: layers }
    } else {
        GodObjectScenario::Borderline {
            primary_recommendation: "Consider extracting groups of related methods".into(),
            alternatives: vec![],
        }
    }
}
```

#### 3. Recommendation Generation

```rust
pub fn generate_recommendation(
    context: &RecommendationContext,
    scenario: &GodObjectScenario,
) -> Recommendation {
    match scenario {
        GodObjectScenario::CohesiveLarge { long_methods } => {
            let methods_list = long_methods.join(", ");
            Recommendation {
                primary_action: format!(
                    "Refactor internally: break down long methods ({})",
                    methods_list
                ),
                rationale: format!(
                    "Good cohesion ({:.0}%) suggests unified domain. \
                     Focus on method extraction, not struct splitting.",
                    context.cohesion_score * 100.0
                ),
                suggested_extractions: long_methods.iter()
                    .map(|m| format!("{} → extract helper functions", m))
                    .collect(),
                effort_estimate: "Low - internal refactoring only".into(),
            }
        }

        GodObjectScenario::MultiDomain { domains } => {
            let domain_summaries: Vec<String> = domains.iter()
                .map(|d| format!(
                    "{} ({} methods → {})",
                    d.domain_name,
                    d.methods.len(),
                    d.suggested_module_name
                ))
                .collect();

            Recommendation {
                primary_action: format!(
                    "Split into {} domain-specific modules: {}",
                    domains.len(),
                    domain_summaries.join(", ")
                ),
                rationale: format!(
                    "Low cohesion ({:.0}%) with {} distinct domains. \
                     Methods don't share common purpose.",
                    context.cohesion_score * 100.0,
                    domains.len()
                ),
                suggested_extractions: domains.iter()
                    .map(|d| format!(
                        "mod {}: [{}]",
                        d.suggested_module_name,
                        d.methods.join(", ")
                    ))
                    .collect(),
                effort_estimate: format!("Medium - {} new modules", domains.len()),
            }
        }

        GodObjectScenario::SingleDomainLarge { suggested_layers } => {
            Recommendation {
                primary_action: "Extract by responsibility layer".into(),
                rationale: format!(
                    "Single domain but {} methods. \
                     Consider separating data access, business logic, and coordination.",
                    context.total_methods
                ),
                suggested_extractions: suggested_layers.iter()
                    .map(|l| format!("{}: {}", l.layer_name, l.description))
                    .collect(),
                effort_estimate: "Medium - architectural refactoring".into(),
            }
        }

        GodObjectScenario::Borderline { primary_recommendation, alternatives } => {
            Recommendation {
                primary_action: primary_recommendation.clone(),
                rationale: "Borderline case - multiple valid approaches".into(),
                suggested_extractions: alternatives.clone(),
                effort_estimate: "Variable".into(),
            }
        }
    }
}

fn generate_module_name(domain_name: &str) -> String {
    // Convert domain name to valid Rust module name
    let sanitized = domain_name
        .to_lowercase()
        .replace(" ", "_")
        .replace("-", "_");

    // Add suffix based on domain type
    if sanitized.ends_with("ing") {
        format!("{}_module", sanitized)
    } else {
        format!("{}_handler", sanitized)
    }
}
```

#### 4. Integration

Update `generate_recommendation` in `recommendation_generator.rs`:

```rust
pub fn generate_recommendation(
    god_analysis: &GodObjectAnalysis,
    pattern_analysis: Option<&PatternAnalysis>,
) -> Recommendation {
    // Build context from analysis
    let context = RecommendationContext {
        cohesion_score: god_analysis.cohesion_score.unwrap_or(0.5),
        domain_groups: god_analysis.domain_groups.clone(),
        long_methods: identify_long_methods(god_analysis),
        total_methods: god_analysis.method_count,
        substantive_methods: god_analysis.substantive_method_count,
        largest_method_loc: god_analysis.largest_method_loc,
        struct_name: god_analysis.struct_name.clone().unwrap_or_default(),
    };

    // Classify scenario
    let scenario = classify_scenario(&context);

    // Generate appropriate recommendation
    generate_recommendation(&context, &scenario)
}
```

### Recommendation Output Format

```rust
#[derive(Debug, Clone)]
pub struct Recommendation {
    /// Primary action to take
    pub primary_action: String,
    /// Why this recommendation is made
    pub rationale: String,
    /// Specific extractions or refactorings suggested
    pub suggested_extractions: Vec<String>,
    /// Estimated effort level
    pub effort_estimate: String,
}
```

## Dependencies

- **Prerequisites**:
  - Spec 206: Cohesion Gate (provides cohesion score)
  - Spec 208: Domain-Aware Grouping (provides domain groups)
- **Affected Components**:
  - `recommendation_generator.rs`: Major rewrite
  - `god_object.rs` (unified builder): Update recommendation rendering

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_classify_cohesive_large() {
    let context = RecommendationContext {
        cohesion_score: 0.75,
        domain_groups: [("module".into(), vec!["a".into(), "b".into()])].into(),
        long_methods: vec![LongMethodInfo { name: "analyze".into(), line_count: 50, complexity: 15 }],
        total_methods: 15,
        substantive_methods: 10,
        largest_method_loc: 50,
        struct_name: "ModuleTracker".into(),
    };

    let scenario = classify_scenario(&context);
    assert!(matches!(scenario, GodObjectScenario::CohesiveLarge { .. }));
}

#[test]
fn test_classify_multi_domain() {
    let context = RecommendationContext {
        cohesion_score: 0.15,
        domain_groups: [
            ("parsing".into(), vec!["parse_json".into()]),
            ("rendering".into(), vec!["render_html".into()]),
            ("validation".into(), vec!["validate_email".into()]),
        ].into_iter().collect(),
        long_methods: vec![],
        total_methods: 20,
        substantive_methods: 20,
        largest_method_loc: 30,
        struct_name: "AppManager".into(),
    };

    let scenario = classify_scenario(&context);
    assert!(matches!(scenario, GodObjectScenario::MultiDomain { .. }));
}

#[test]
fn test_generate_module_name() {
    assert_eq!(generate_module_name("Parsing"), "parsing_module");
    assert_eq!(generate_module_name("data access"), "data_access_handler");
    assert_eq!(generate_module_name("Validation"), "validation_handler");
}

#[test]
fn test_recommendation_for_cohesive_struct() {
    let context = /* cohesive context */;
    let scenario = GodObjectScenario::CohesiveLarge {
        long_methods: vec!["analyze_workspace".into()],
    };

    let rec = generate_recommendation(&context, &scenario);

    assert!(rec.primary_action.contains("Refactor internally"));
    assert!(rec.rationale.contains("cohesion"));
    assert!(!rec.primary_action.contains("sub-orchestrators"));
}
```

### Integration Tests

```rust
#[test]
fn test_cross_module_tracker_recommendation() {
    let analysis = analyze_cross_module_tracker();

    // Should recommend internal refactoring, not splitting
    assert!(
        analysis.recommendation.primary_action.contains("refactor")
            || analysis.recommendation.primary_action.contains("method"),
        "Cohesive tracker should get internal refactoring recommendation"
    );
    assert!(
        !analysis.recommendation.primary_action.contains("sub-orchestrator"),
        "Should not suggest sub-orchestrators for cohesive struct"
    );
}
```

## Documentation Requirements

- **Code Documentation**: Document recommendation scenarios in recommendation_generator.rs
- **User Documentation**: Explain how recommendation types map to code issues

## Implementation Notes

1. **Graceful Degradation**: If context is incomplete, fall back to simpler recommendations
2. **Length Limits**: Keep recommendations under 200 words
3. **Valid Identifiers**: Ensure generated module names are valid Rust
4. **Determinism**: Same analysis should produce same recommendation

## Migration and Compatibility

- Recommendation text will change significantly
- Old recommendation format was single string; new format is structured
- Consider providing both for transition period

## Estimated Effort

- Implementation: ~3 hours
- Testing: ~1.5 hours
- Documentation: ~0.5 hours
- Total: ~5 hours
