---
number: 152
title: Domain Diversity Metrics for Struct Splits
category: optimization
priority: high
status: draft
dependencies: [140]
created: 2025-10-28
---

# Specification 152: Domain Diversity Metrics for Struct Splits

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 140 (Domain-Based Struct Split Recommendations)

## Context

Spec 140 (Domain-Based Struct Split Recommendations) is implemented and working for config.rs (#4 in latest output):

```
#4 SCORE: 100 [CRITICAL - FILE - GOD OBJECT]
└─ ./src/config.rs (2732 lines, 217 functions)
└─ WHY: This module contains 217 module functions across 1 responsibilities...
└─ ACTION: URGENT: 2732 lines, 217 functions! Split by data flow...

  - RECOMMENDED SPLITS (5 modules):
  -  [M] config/misc.rs.rs - misc (0 methods, ~25 lines)
       -> Structs: SeverityOverride, LanguageFeatures (2 structs)
  -  [M] config/thresholds.rs.rs - thresholds (0 methods, ~116 lines)
       -> Structs: ValidationThresholds, GodObjectThresholds, ThresholdsConfig (3 structs)
  -  [M] config/detection.rs.rs - detection (0 methods, ~81 lines)
       -> Structs: AccessorDetectionConfig, OrchestratorDetectionConfig, ... (3 structs)
  -  [M] config/core_config.rs.rs - core_config (0 methods, ~400 lines)
       -> Structs: DataFlowClassificationConfig, ContextRuleConfig, ... (17 structs)
  -  [M] config/scoring.rs.rs - scoring (0 methods, ~119 lines)
       -> Structs: RoleMultipliers, ScoringWeights, ... (5 structs)
```

This is excellent - struct-based domain grouping is working. However, **crucial metrics are missing**:

1. **Domain diversity**: How many distinct domains are mixed? (Expected: ~5-7 based on splits)
2. **Cross-domain severity**: Is this CRITICAL mixing or mild grouping?
3. **Spec 140 reference**: This recommendation should cite Spec 140 methodology
4. **Domain confidence**: How confident is domain classification?
5. **Alternative groupings**: Could structs be grouped differently?

**Current WHY statement** says "across 1 responsibilities", but the splits show **5 distinct domains**. This is contradictory and confusing.

Per Spec 140, domain-based splits should include:
- Domain count
- Domain diversity score
- Cross-domain mixing severity (CRITICAL/HIGH/MEDIUM/LOW per Spec 140)
- Evidence for domain classification

## Objective

Add comprehensive domain diversity metrics to struct-based split recommendations, showing:
1. Total number of distinct domains detected
2. Domain diversity score (entropy-based measure of how mixed domains are)
3. Cross-domain mixing severity (per Spec 140 severity levels)
4. Confidence in domain classifications
5. Domain distribution (how many structs per domain)
6. Evidence for why structs were grouped into domains

This will provide clarity on **why** config.rs needs splitting and **how severe** the cross-domain mixing is.

## Requirements

### Functional Requirements

**Domain Diversity Metrics**:
- Count distinct domains across all structs
- Calculate domain diversity score (0.0 = all same domain, 1.0 = maximum diversity)
- Show struct distribution across domains
- Identify largest and smallest domain groups

**Cross-Domain Severity**:
- Apply Spec 140 severity classification:
  - CRITICAL: God object + 3+ domains, OR 15+ structs + 5+ domains
  - HIGH: 10+ structs + 4+ domains
  - MEDIUM: 8+ structs + 3+ domains
  - LOW: 5+ structs + 3+ domains (informational)
- Display severity prominently in output
- Link severity to urgency of refactoring

**Domain Classification Evidence**:
- Show why each struct was classified into its domain
- Pattern matching evidence (name patterns, field types)
- Confidence scores per struct
- Alternative domain classifications if ambiguous

**Struct Distribution**:
- List number of structs per domain
- Show largest domain (potential for further splitting)
- Show smallest domain (might merge or reorganize)
- Identify outlier structs (don't fit any domain well)

**Spec 140 Integration**:
- Explicitly reference Spec 140 in output
- Use Spec 140 terminology and metrics
- Apply Spec 140 severity thresholds
- Show Spec 140 decision logic (why CRITICAL vs HIGH)

### Non-Functional Requirements

- **Performance**: Domain metric calculation adds <5% overhead
- **Accuracy**: Domain classification >80% accurate (per Spec 140)
- **Clarity**: Metrics easy to understand for users
- **Actionability**: Severity indicates urgency

## Acceptance Criteria

- [ ] Domain count displayed for struct-based splits
- [ ] Domain diversity score calculated and shown
- [ ] Cross-domain severity (CRITICAL/HIGH/MEDIUM/LOW) displayed prominently
- [ ] Spec 140 referenced in output for struct splits
- [ ] Struct distribution across domains shown
- [ ] Domain classification evidence provided
- [ ] "Across 1 responsibilities" corrected to actual domain count
- [ ] Confidence scores shown for domain classifications
- [ ] Severity justification explained (why CRITICAL vs HIGH)
- [ ] Test suite validates domain metric calculations

## Technical Details

### Implementation Approach

**Phase 1: Domain Diversity Calculation**

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DomainDiversityMetrics {
    pub total_structs: usize,
    pub domain_count: usize,
    pub domain_distribution: HashMap<String, Vec<String>>,  // domain → struct names
    pub diversity_score: f64,  // 0.0-1.0, entropy-based
    pub severity: CrossDomainSeverity,
    pub is_god_object: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossDomainSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl DomainDiversityMetrics {
    pub fn from_struct_classifications(
        classifications: &[StructDomainClassification],
        is_god_object: bool,
    ) -> Self {
        let total_structs = classifications.len();

        // Group structs by domain
        let mut domain_distribution: HashMap<String, Vec<String>> = HashMap::new();

        for classification in classifications {
            domain_distribution
                .entry(classification.domain.clone())
                .or_insert_with(Vec::new)
                .push(classification.struct_name.clone());
        }

        let domain_count = domain_distribution.len();

        // Calculate diversity score (Shannon entropy normalized)
        let diversity_score = calculate_domain_entropy(&domain_distribution, total_structs);

        // Determine severity (per Spec 140)
        let severity = determine_cross_domain_severity(
            total_structs,
            domain_count,
            is_god_object,
        );

        DomainDiversityMetrics {
            total_structs,
            domain_count,
            domain_distribution,
            diversity_score,
            severity,
            is_god_object,
        }
    }

    pub fn largest_domain(&self) -> Option<(&String, usize)> {
        self.domain_distribution.iter()
            .map(|(domain, structs)| (domain, structs.len()))
            .max_by_key(|(_, count)| *count)
    }

    pub fn smallest_domain(&self) -> Option<(&String, usize)> {
        self.domain_distribution.iter()
            .map(|(domain, structs)| (domain, structs.len()))
            .min_by_key(|(_, count)| *count)
    }
}

fn calculate_domain_entropy(
    distribution: &HashMap<String, Vec<String>>,
    total: usize,
) -> f64 {
    if total == 0 {
        return 0.0;
    }

    let mut entropy = 0.0;

    for structs in distribution.values() {
        let p = structs.len() as f64 / total as f64;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }

    // Normalize to 0.0-1.0 range
    let max_entropy = (distribution.len() as f64).log2();
    if max_entropy > 0.0 {
        entropy / max_entropy
    } else {
        0.0
    }
}

fn determine_cross_domain_severity(
    struct_count: usize,
    domain_count: usize,
    is_god_object: bool,
) -> CrossDomainSeverity {
    // Per Spec 140 severity levels

    // CRITICAL: God object with cross-domain mixing
    if is_god_object && domain_count >= 3 {
        return CrossDomainSeverity::Critical;
    }

    // CRITICAL: Massive cross-domain mixing
    if struct_count > 15 && domain_count >= 5 {
        return CrossDomainSeverity::Critical;
    }

    // HIGH: Significant cross-domain issues
    if struct_count >= 10 && domain_count >= 4 {
        return CrossDomainSeverity::High;
    }

    // MEDIUM: Proactive improvement opportunity
    if struct_count >= 8 || domain_count >= 3 {
        return CrossDomainSeverity::Medium;
    }

    // LOW: Informational only
    CrossDomainSeverity::Low
}
```

**Phase 2: Domain Classification Evidence**

```rust
#[derive(Debug, Clone)]
pub struct StructDomainClassification {
    pub struct_name: String,
    pub domain: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub alternatives: Vec<(String, f64)>,  // (domain, confidence)
}

impl StructDomainClassification {
    pub fn format_evidence(&self) -> String {
        let mut output = format!(
            "{} → {} domain ({:.0}% confidence)\n",
            self.struct_name,
            self.domain,
            self.confidence * 100.0
        );

        if !self.evidence.is_empty() {
            output.push_str(&format!(
                "  Evidence: {}\n",
                self.evidence.join(", ")
            ));
        }

        if !self.alternatives.is_empty() && self.alternatives[0].1 > 0.50 {
            output.push_str(&format!(
                "  Alternative: {} ({:.0}%)\n",
                self.alternatives[0].0,
                self.alternatives[0].1 * 100.0
            ));
        }

        output
    }
}
```

**Phase 3: Output Formatting**

```rust
impl DomainDiversityMetrics {
    pub fn format_for_output(&self) -> String {
        let mut output = String::new();

        // Header with severity
        output.push_str(&format!(
            "\nDOMAIN DIVERSITY ANALYSIS (Spec 140):\n\
             Severity: {} - {} structs across {} domains\n\n",
            self.format_severity(),
            self.total_structs,
            self.domain_count
        ));

        // Severity justification
        output.push_str(&self.format_severity_justification());
        output.push_str("\n\n");

        // Domain distribution
        output.push_str("Domain Distribution:\n");

        let mut domains: Vec<_> = self.domain_distribution.iter().collect();
        domains.sort_by_key(|(_, structs)| std::cmp::Reverse(structs.len()));

        for (domain, structs) in domains {
            output.push_str(&format!(
                "  - {}: {} structs ({:.0}%)\n",
                domain,
                structs.len(),
                (structs.len() as f64 / self.total_structs as f64) * 100.0
            ));

            // Show first few struct names
            let examples: Vec<_> = structs.iter().take(3).collect();
            output.push_str(&format!(
                "    Examples: {}{}\n",
                examples.iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                if structs.len() > 3 {
                    format!(", ... +{} more", structs.len() - 3)
                } else {
                    String::new()
                }
            ));
        }

        // Diversity score
        output.push_str(&format!(
            "\nDiversity Score: {:.2} (0.0 = homogeneous, 1.0 = maximum diversity)\n",
            self.diversity_score
        ));

        // Largest/smallest domains
        if let Some((domain, count)) = self.largest_domain() {
            output.push_str(&format!(
                "Largest domain: {} ({} structs) - may need further splitting\n",
                domain, count
            ));
        }

        if let Some((domain, count)) = self.smallest_domain() {
            if count == 1 {
                output.push_str(&format!(
                    "Singleton domain: {} (1 struct) - may merge or recategorize\n",
                    domain
                ));
            }
        }

        output
    }

    fn format_severity(&self) -> &str {
        match self.severity {
            CrossDomainSeverity::Critical => "CRITICAL",
            CrossDomainSeverity::High => "HIGH",
            CrossDomainSeverity::Medium => "MEDIUM",
            CrossDomainSeverity::Low => "LOW",
        }
    }

    fn format_severity_justification(&self) -> String {
        match self.severity {
            CrossDomainSeverity::Critical if self.is_god_object => {
                format!(
                    "  Reason: God object with {} domains detected (Spec 140: CRITICAL threshold)\n\
                     → URGENT: Violates single responsibility principle at module level",
                    self.domain_count
                )
            }
            CrossDomainSeverity::Critical => {
                format!(
                    "  Reason: {} structs across {} domains (Spec 140: CRITICAL threshold)\n\
                     → URGENT: Massive cross-domain mixing",
                    self.total_structs,
                    self.domain_count
                )
            }
            CrossDomainSeverity::High => {
                format!(
                    "  Reason: {} structs across {} domains (Spec 140: HIGH threshold)\n\
                     → HIGH PRIORITY: Significant organizational debt",
                    self.total_structs,
                    self.domain_count
                )
            }
            CrossDomainSeverity::Medium => {
                format!(
                    "  Reason: {} structs across {} domains (Spec 140: MEDIUM threshold)\n\
                     → PROACTIVE: Good time to organize before it grows",
                    self.total_structs,
                    self.domain_count
                )
            }
            CrossDomainSeverity::Low => {
                format!(
                    "  Reason: {} structs across {} domains (Spec 140: LOW threshold)\n\
                     → INFORMATIONAL: Minor organizational improvement opportunity",
                    self.total_structs,
                    self.domain_count
                )
            }
        }
    }
}
```

**Phase 4: Integration with Recommendation Output**

```rust
// In src/priority/formatter.rs or src/organization/god_object_detector.rs

pub fn format_config_recommendation_with_domain_metrics(
    recommendation: &ModuleSplitRecommendation,
    domain_metrics: &DomainDiversityMetrics,
    classifications: &[StructDomainClassification],
) -> String {
    let mut output = String::new();

    // Existing recommendation header
    output.push_str(&recommendation.summary);
    output.push_str("\n");

    // NEW: Domain diversity metrics
    output.push_str(&domain_metrics.format_for_output());

    // Recommended splits
    output.push_str("\n  - RECOMMENDED SPLITS:\n");
    for split in &recommendation.proposed_modules {
        output.push_str(&format!("  -  [{}] {}\n", split.priority, split.name));

        // Show structs in this split
        if let Some(structs) = domain_metrics.domain_distribution.get(&split.domain) {
            output.push_str(&format!(
                "       -> Structs: {} ({} structs)\n",
                structs.join(", "),
                structs.len()
            ));
        }
    }

    // Optional: Show classification evidence in verbose mode
    if recommendation.verbose {
        output.push_str("\n  - DOMAIN CLASSIFICATION EVIDENCE:\n");
        for classification in classifications.iter().take(10) {
            output.push_str(&format!("    {}", classification.format_evidence()));
        }
    }

    output
}
```

**Phase 5: Fix "Across 1 responsibilities" Issue**

```rust
// In god_object_detector.rs or similar

pub fn generate_why_statement(
    file_analysis: &FileAnalysis,
    domain_metrics: &Option<DomainDiversityMetrics>,
) -> String {
    // For struct-heavy files with domain metrics
    if let Some(metrics) = domain_metrics {
        return format!(
            "This module contains {} structs across {} distinct domains. \
             Cross-domain mixing (Severity: {}) violates single responsibility principle.",
            metrics.total_structs,
            metrics.domain_count,
            metrics.format_severity()
        );
    }

    // For function-heavy files
    if file_analysis.module_functions.len() > file_analysis.total_struct_methods() {
        return format!(
            "This module contains {} module functions across {} responsibilities. \
             Large modules with many diverse functions are difficult to navigate, understand, and maintain.",
            file_analysis.module_functions.len(),
            file_analysis.responsibility_count
        );
    }

    // For struct methods
    format!(
        "This struct violates single responsibility principle with {} methods and {} fields \
         across {} distinct responsibilities. High coupling and low cohesion make it difficult to maintain and test.",
        file_analysis.total_methods,
        file_analysis.total_fields,
        file_analysis.responsibility_count
    )
}
```

### Architecture Changes

**New Module**: `src/organization/domain_diversity.rs`
- Domain diversity calculation
- Cross-domain severity determination
- Distribution analysis
- Evidence formatting

**Modified Module**: `src/organization/god_object_detector.rs`
- Calculate domain metrics for struct-heavy files
- Pass metrics to output formatter
- Fix responsibility count for structs (use domain count, not hardcoded "1")

**Modified Module**: `src/priority/formatter.rs`
- Integrate domain diversity metrics into output
- Format severity prominently
- Show domain distribution

## Dependencies

- **Prerequisites**: Spec 140 (Domain-Based Struct Splits)
- **Affected Components**:
  - `src/organization/god_object_detector.rs` - metric calculation
  - `src/priority/formatter.rs` - output formatting
  - `src/organization/domain_classifier.rs` - domain classification (existing)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_domain_diversity() {
        let classifications = vec![
            StructDomainClassification {
                struct_name: "ScoringWeights".into(),
                domain: "scoring".into(),
                confidence: 0.90,
                evidence: vec![],
                alternatives: vec![],
            },
            StructDomainClassification {
                struct_name: "ThresholdsConfig".into(),
                domain: "thresholds".into(),
                confidence: 0.85,
                evidence: vec![],
                alternatives: vec![],
            },
            StructDomainClassification {
                struct_name: "RoleMultipliers".into(),
                domain: "scoring".into(),
                confidence: 0.80,
                evidence: vec![],
                alternatives: vec![],
            },
        ];

        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false);

        assert_eq!(metrics.total_structs, 3);
        assert_eq!(metrics.domain_count, 2);  // scoring, thresholds
        assert!(metrics.diversity_score > 0.0);
        assert!(metrics.diversity_score < 1.0);  // Not maximum diversity
    }

    #[test]
    fn critical_severity_for_god_object_with_domains() {
        let classifications = create_test_classifications(10, 3);  // 10 structs, 3 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(
            &classifications,
            true  // is_god_object
        );

        assert_eq!(metrics.severity, CrossDomainSeverity::Critical);
    }

    #[test]
    fn critical_severity_for_massive_mixing() {
        let classifications = create_test_classifications(20, 5);  // 20 structs, 5 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(
            &classifications,
            false  // not god object, but still critical
        );

        assert_eq!(metrics.severity, CrossDomainSeverity::Critical);
    }

    #[test]
    fn high_severity() {
        let classifications = create_test_classifications(12, 4);  // 12 structs, 4 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false);

        assert_eq!(metrics.severity, CrossDomainSeverity::High);
    }
}
```

### Integration Tests

```rust
#[test]
fn config_rs_shows_domain_metrics() {
    let analysis = analyze_file("src/config.rs");
    let output = format_recommendation(&analysis);

    // Should contain domain metrics
    assert!(output.contains("DOMAIN DIVERSITY ANALYSIS"));
    assert!(output.contains("Spec 140"));

    // Should show severity
    assert!(
        output.contains("CRITICAL") ||
        output.contains("HIGH") ||
        output.contains("MEDIUM")
    );

    // Should show domain count
    assert!(output.contains("domains"));

    // Should NOT say "across 1 responsibilities"
    assert!(!output.contains("across 1 responsibilities"));
}
```

## Documentation Requirements

### User Documentation

Update README.md:
```markdown
## Domain-Based Struct Analysis (Spec 140)

For configuration files and struct-heavy modules, debtmap analyzes domain diversity:

**Domain Diversity Metrics**:
- Total structs and distinct domains detected
- Domain distribution (structs per domain)
- Diversity score (entropy-based, 0.0-1.0)

**Cross-Domain Severity** (Spec 140):
- CRITICAL: God object + 3+ domains, OR 15+ structs + 5+ domains
- HIGH: 10+ structs + 4+ domains
- MEDIUM: 8+ structs + 3+ domains
- LOW: 5+ structs + 3+ domains (informational)

**Example Output**:
```
DOMAIN DIVERSITY ANALYSIS (Spec 140):
Severity: CRITICAL - 30 structs across 5 domains

  Reason: 30 structs across 5 domains (Spec 140: CRITICAL threshold)
  → URGENT: Massive cross-domain mixing

Domain Distribution:
  - scoring: 5 structs (17%)
    Examples: ScoringWeights, RoleMultipliers, ComplexityWeights
  - thresholds: 3 structs (10%)
    Examples: ThresholdsConfig, ValidationThresholds, GodObjectThresholds
  - detection: 4 structs (13%)
    Examples: AccessorDetectionConfig, OrchestratorDetectionConfig, ...
  - core_config: 15 structs (50%)
    Examples: DebtmapConfig, OutputConfig, ClassificationConfig, ...
  - misc: 3 structs (10%)
    Examples: SeverityOverride, LanguageFeatures

Diversity Score: 0.78 (0.0 = homogeneous, 1.0 = maximum diversity)
Largest domain: core_config (15 structs) - may need further splitting
```
```

## Implementation Notes

### Domain Entropy Calculation

Shannon entropy normalized by maximum possible entropy:

```rust
H = -Σ p_i * log2(p_i)  // Shannon entropy
H_max = log2(n)         // Maximum entropy (n = number of domains)
diversity = H / H_max   // Normalized to 0.0-1.0
```

For config.rs with 5 domains:
- If evenly distributed (6 structs each): diversity ≈ 1.0
- If one dominant domain (20 structs), others small: diversity ≈ 0.6
- If all in one domain: diversity = 0.0

### Performance Optimization

Domain metrics calculated once during analysis, cached for display:

```rust
pub struct FileAnalysisCache {
    domain_metrics: OnceCell<Option<DomainDiversityMetrics>>,
}

impl FileAnalysisCache {
    pub fn get_domain_metrics(&self, file: &FileAnalysis) -> Option<&DomainDiversityMetrics> {
        self.domain_metrics.get_or_init(|| {
            if file.is_struct_heavy() {
                Some(calculate_domain_diversity(file))
            } else {
                None
            }
        }).as_ref()
    }
}
```

## Migration and Compatibility

### Backward Compatibility

- Domain metrics are additions to existing output
- No breaking changes to output format
- Only affects struct-heavy files (config.rs, similar files)

## Expected Impact

### Output Quality

**Before (current)**:
```
#4 SCORE: 100 [CRITICAL - FILE - GOD OBJECT]
└─ ./src/config.rs (2732 lines, 217 functions)
└─ WHY: This module contains 217 module functions across 1 responsibilities...
```
**Problem**: Says "1 responsibilities" but recommends 5 splits - contradictory!

**After (with domain metrics)**:
```
#4 SCORE: 100 [CRITICAL - FILE - GOD OBJECT]
└─ ./src/config.rs (2732 lines, 30 structs)
└─ WHY: This module contains 30 structs across 5 distinct domains.
        Cross-domain mixing (Severity: CRITICAL) violates single responsibility principle.

DOMAIN DIVERSITY ANALYSIS (Spec 140):
Severity: CRITICAL - 30 structs across 5 domains

  Reason: 30 structs across 5 domains (Spec 140: CRITICAL threshold)
  → URGENT: Massive cross-domain mixing

Domain Distribution:
  - scoring: 5 structs (17%) [ScoringWeights, RoleMultipliers, ...]
  - thresholds: 3 structs (10%) [ThresholdsConfig, ValidationThresholds, ...]
  - detection: 4 structs (13%) [AccessorDetectionConfig, ...]
  - core_config: 15 structs (50%) [DebtmapConfig, OutputConfig, ...]
  - misc: 3 structs (10%) [SeverityOverride, LanguageFeatures]

Diversity Score: 0.78
Largest domain: core_config (15 structs) - may need further splitting
```

### User Benefits

- **Clarity**: Understand exactly how many domains are mixed
- **Severity awareness**: Know if this is CRITICAL or just informational
- **Spec linkage**: Connect to Spec 140 methodology
- **Distribution insight**: See which domains dominate
- **Actionability**: Largest domain flagged for further attention

## Success Metrics

- [ ] Domain count displayed for all struct-heavy files
- [ ] Cross-domain severity (CRITICAL/HIGH/MEDIUM/LOW) shown
- [ ] Spec 140 referenced in output
- [ ] "Across 1 responsibilities" bug fixed
- [ ] Domain distribution shown with percentages
- [ ] Diversity score calculated and displayed
- [ ] Largest domain identified
- [ ] Severity justification provided
- [ ] User feedback: Metrics make recommendations clearer
