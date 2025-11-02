---
number: 152
title: Domain Diversity Metrics for Struct Splits
category: optimization
priority: high
status: ready
dependencies: [140]
created: 2025-10-28
updated: 2025-11-02
---

# Specification 152: Domain Diversity Metrics for Struct Splits

**Category**: optimization
**Priority**: high
**Status**: ready
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

- **Performance**: Domain metric calculation adds <5% overhead (verified by criterion benchmarks)
- **Memory**: No significant memory regression on large files (>1000 structs)
- **Accuracy**: Domain classification >80% accurate (per Spec 140)
- **Clarity**: Metrics easy to understand for users
- **Actionability**: Severity indicates urgency
- **Error Handling**: Gracefully handle edge cases (empty classifications, single domain, invalid data)

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
- [ ] **Performance**: Benchmark validates <5% overhead claim
- [ ] **Memory**: No significant memory regression on large files (>1000 structs)
- [ ] **Error handling**: Gracefully handles edge cases (empty classifications, single domain, etc.)
- [ ] **Type safety**: DiversityScore newtype enforces 0.0-1.0 bounds
- [ ] **Property tests**: Invariants verified (score bounds, single domain = 0.0, etc.)

## Technical Details

### Implementation Approach

**Phase 1: Type-Safe Domain Diversity Calculation (Functional)**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Context, Result};

/// Type-safe diversity score (0.0 = homogeneous, 1.0 = maximum diversity)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct DiversityScore(f64);

impl DiversityScore {
    pub fn new(value: f64) -> Result<Self> {
        if !(0.0..=1.0).contains(&value) {
            anyhow::bail!("Diversity score must be between 0.0 and 1.0, got {}", value);
        }
        Ok(DiversityScore(value))
    }

    /// Create a diversity score of 0.0 (homogeneous)
    pub fn zero() -> Self {
        DiversityScore(0.0)
    }

    pub fn as_f64(&self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct DomainDiversityMetrics {
    pub total_structs: usize,
    pub domain_count: usize,
    pub domain_distribution: HashMap<Arc<str>, Vec<Arc<str>>>,  // Shared ownership
    pub diversity_score: DiversityScore,
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
    /// Pure functional constructor from struct classifications
    pub fn from_struct_classifications(
        classifications: &[StructDomainClassification],
        is_god_object: bool,
    ) -> Result<Self> {
        let total_structs = classifications.len();

        // Handle edge case: no structs
        if total_structs == 0 {
            return Ok(DomainDiversityMetrics {
                total_structs: 0,
                domain_count: 0,
                domain_distribution: HashMap::new(),
                diversity_score: DiversityScore::zero(),
                severity: CrossDomainSeverity::Low,
                is_god_object,
            });
        }

        // Functional grouping by domain (immutable transformation)
        let domain_distribution = group_structs_by_domain(classifications);
        let domain_count = domain_distribution.len();

        // Pure calculations
        let diversity_score = calculate_domain_entropy(&domain_distribution, total_structs)
            .context("Failed to calculate domain entropy")?;

        let severity = determine_cross_domain_severity(
            total_structs,
            domain_count,
            is_god_object,
        );

        Ok(DomainDiversityMetrics {
            total_structs,
            domain_count,
            domain_distribution,
            diversity_score,
            severity,
            is_god_object,
        })
    }

    pub fn largest_domain(&self) -> Option<(&Arc<str>, usize)> {
        self.domain_distribution
            .iter()
            .map(|(domain, structs)| (domain, structs.len()))
            .max_by_key(|(_, count)| *count)
    }

    pub fn smallest_domain(&self) -> Option<(&Arc<str>, usize)> {
        self.domain_distribution
            .iter()
            .map(|(domain, structs)| (domain, structs.len()))
            .min_by_key(|(_, count)| *count)
    }
}

/// Pure function: Group structs by domain using functional composition
fn group_structs_by_domain(
    classifications: &[StructDomainClassification]
) -> HashMap<Arc<str>, Vec<Arc<str>>> {
    classifications
        .iter()
        .fold(HashMap::new(), |mut acc, classification| {
            acc.entry(Arc::from(classification.domain.as_str()))
                .or_insert_with(Vec::new)
                .push(Arc::from(classification.struct_name.as_str()));
            acc
        })
}

/// Pure function: Calculate Shannon entropy normalized to 0.0-1.0
fn calculate_domain_entropy(
    distribution: &HashMap<Arc<str>, Vec<Arc<str>>>,
    total: usize,
) -> Result<DiversityScore> {
    // Edge cases
    if total == 0 || distribution.is_empty() {
        return Ok(DiversityScore::zero());
    }

    // Single domain = zero diversity
    if distribution.len() == 1 {
        return Ok(DiversityScore::zero());
    }

    // Calculate Shannon entropy: H = -Σ p_i * log2(p_i)
    let entropy: f64 = distribution
        .values()
        .map(|structs| {
            let p = structs.len() as f64 / total as f64;
            if p > 0.0 {
                -p * p.log2()
            } else {
                0.0
            }
        })
        .sum();

    // Normalize to 0.0-1.0 range: diversity = H / H_max
    let max_entropy = (distribution.len() as f64).log2();
    let normalized = if max_entropy > 0.0 {
        entropy / max_entropy
    } else {
        0.0
    };

    DiversityScore::new(normalized)
        .context("Entropy calculation produced invalid diversity score")
}

/// Pure function: Determine severity based on Spec 140 thresholds
fn determine_cross_domain_severity(
    struct_count: usize,
    domain_count: usize,
    is_god_object: bool,
) -> CrossDomainSeverity {
    match (is_god_object, struct_count, domain_count) {
        // CRITICAL: God object with cross-domain mixing
        (true, _, d) if d >= 3 => CrossDomainSeverity::Critical,

        // CRITICAL: Massive cross-domain mixing
        (_, s, d) if s > 15 && d >= 5 => CrossDomainSeverity::Critical,

        // HIGH: Significant cross-domain issues
        (_, s, d) if s >= 10 && d >= 4 => CrossDomainSeverity::High,

        // MEDIUM: Proactive improvement opportunity
        (_, s, d) if s >= 8 || d >= 3 => CrossDomainSeverity::Medium,

        // LOW: Informational only
        _ => CrossDomainSeverity::Low,
    }
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
            self.diversity_score.as_f64()
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

Public API:
```rust
// Core types (public)
pub struct DomainDiversityMetrics { ... }
pub struct DiversityScore(f64);
pub enum CrossDomainSeverity { Critical, High, Medium, Low }
pub struct StructDomainClassification { ... }

// Public constructors and methods
impl DomainDiversityMetrics {
    pub fn from_struct_classifications(...) -> Result<Self>;
    pub fn largest_domain(&self) -> Option<(&Arc<str>, usize)>;
    pub fn smallest_domain(&self) -> Option<(&Arc<str>, usize)>;
    pub fn format_for_output(&self) -> String;
}

impl DiversityScore {
    pub fn new(value: f64) -> Result<Self>;
    pub fn zero() -> Self;
    pub fn as_f64(&self) -> f64;
}

// Private helpers (module-internal)
fn group_structs_by_domain(...) -> HashMap<Arc<str>, Vec<Arc<str>>>;
fn calculate_domain_entropy(...) -> Result<DiversityScore>;
fn determine_cross_domain_severity(...) -> CrossDomainSeverity;
```

Responsibilities:
- Domain diversity calculation (pure functions)
- Cross-domain severity determination (pure functions)
- Distribution analysis (pure functions)
- Evidence formatting (output formatting)

**Modified Module**: `src/organization/god_object_detector.rs`
- Calculate domain metrics for struct-heavy files
- Pass metrics to output formatter
- Fix responsibility count for structs (use domain count, not hardcoded "1")
- Import and use `domain_diversity::DomainDiversityMetrics`

**Modified Module**: `src/priority/formatter.rs`
- Integrate domain diversity metrics into output
- Format severity prominently
- Show domain distribution
- Call `DomainDiversityMetrics::format_for_output()`

**Dependencies**:
```toml
[dependencies]
# Existing dependencies...

# No new external dependencies required
# Uses existing: anyhow, std::collections::HashMap, std::sync::Arc
```

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
    use proptest::prelude::*;
    use pretty_assertions::assert_eq;

    // Test helper: Create evenly distributed classifications
    fn create_test_classifications(struct_count: usize, domain_count: usize) -> Vec<StructDomainClassification> {
        let domains: Vec<String> = (0..domain_count)
            .map(|i| format!("domain_{}", i))
            .collect();

        (0..struct_count)
            .map(|i| StructDomainClassification {
                struct_name: format!("Struct{}", i),
                domain: domains[i % domain_count].clone(),
                confidence: 0.8,
                evidence: vec![],
                alternatives: vec![],
            })
            .collect()
    }

    // Test helper: Create all structs in single domain
    fn create_uniform_domain_classifications(struct_count: usize) -> Vec<StructDomainClassification> {
        (0..struct_count)
            .map(|i| StructDomainClassification {
                struct_name: format!("Struct{}", i),
                domain: "single_domain".to_string(),
                confidence: 0.9,
                evidence: vec![],
                alternatives: vec![],
            })
            .collect()
    }

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

        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
            .expect("Failed to create metrics");

        assert_eq!(metrics.total_structs, 3);
        assert_eq!(metrics.domain_count, 2);  // scoring, thresholds
        assert!(metrics.diversity_score.as_f64() > 0.0);
        assert!(metrics.diversity_score.as_f64() < 1.0);  // Not maximum diversity
    }

    #[test]
    fn empty_classifications_handled_gracefully() {
        let classifications: Vec<StructDomainClassification> = vec![];
        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
            .expect("Should handle empty classifications");

        assert_eq!(metrics.total_structs, 0);
        assert_eq!(metrics.domain_count, 0);
        assert_eq!(metrics.diversity_score.as_f64(), 0.0);
        assert_eq!(metrics.severity, CrossDomainSeverity::Low);
    }

    #[test]
    fn single_domain_zero_diversity() {
        let classifications = create_uniform_domain_classifications(10);
        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
            .expect("Failed to create metrics");

        assert_eq!(metrics.domain_count, 1);
        assert_eq!(metrics.diversity_score.as_f64(), 0.0);  // Single domain = 0.0 diversity
    }

    #[test]
    fn diversity_score_type_safety() {
        // Valid scores
        assert!(DiversityScore::new(0.0).is_ok());
        assert!(DiversityScore::new(0.5).is_ok());
        assert!(DiversityScore::new(1.0).is_ok());

        // Invalid scores
        assert!(DiversityScore::new(-0.1).is_err());
        assert!(DiversityScore::new(1.1).is_err());
        assert!(DiversityScore::new(f64::NAN).is_err());
    }

    #[test]
    fn critical_severity_for_god_object_with_domains() {
        let classifications = create_test_classifications(10, 3);  // 10 structs, 3 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(
            &classifications,
            true  // is_god_object
        ).expect("Failed to create metrics");

        assert_eq!(metrics.severity, CrossDomainSeverity::Critical);
    }

    #[test]
    fn critical_severity_for_massive_mixing() {
        let classifications = create_test_classifications(20, 5);  // 20 structs, 5 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(
            &classifications,
            false  // not god object, but still critical
        ).expect("Failed to create metrics");

        assert_eq!(metrics.severity, CrossDomainSeverity::Critical);
    }

    #[test]
    fn high_severity() {
        let classifications = create_test_classifications(12, 4);  // 12 structs, 4 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
            .expect("Failed to create metrics");

        assert_eq!(metrics.severity, CrossDomainSeverity::High);
    }

    #[test]
    fn medium_severity() {
        let classifications = create_test_classifications(8, 3);  // 8 structs, 3 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
            .expect("Failed to create metrics");

        assert_eq!(metrics.severity, CrossDomainSeverity::Medium);
    }

    #[test]
    fn low_severity() {
        let classifications = create_test_classifications(4, 2);  // 4 structs, 2 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
            .expect("Failed to create metrics");

        assert_eq!(metrics.severity, CrossDomainSeverity::Low);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn diversity_score_always_bounded(
            struct_count in 1usize..100,
            domain_count in 1usize..20
        ) {
            let classifications = create_test_classifications(struct_count, domain_count);
            let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
                .expect("Failed to create metrics");

            prop_assert!(metrics.diversity_score.as_f64() >= 0.0);
            prop_assert!(metrics.diversity_score.as_f64() <= 1.0);
        }

        #[test]
        fn single_domain_always_zero_diversity(struct_count in 1usize..100) {
            let classifications = create_uniform_domain_classifications(struct_count);
            let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
                .expect("Failed to create metrics");

            prop_assert_eq!(metrics.diversity_score.as_f64(), 0.0);
            prop_assert_eq!(metrics.domain_count, 1);
        }

        #[test]
        fn struct_count_matches_input(
            struct_count in 0usize..100,
            domain_count in 1usize..20
        ) {
            let classifications = if struct_count == 0 {
                vec![]
            } else {
                create_test_classifications(struct_count, domain_count)
            };

            let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
                .expect("Failed to create metrics");

            prop_assert_eq!(metrics.total_structs, struct_count);
        }

        #[test]
        fn god_object_with_multiple_domains_is_critical(
            struct_count in 5usize..100,
            domain_count in 3usize..20
        ) {
            let classifications = create_test_classifications(struct_count, domain_count);
            let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, true)
                .expect("Failed to create metrics");

            prop_assert_eq!(metrics.severity, CrossDomainSeverity::Critical);
        }
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

### Performance Benchmarks

Use criterion for performance validation:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_domain_diversity_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("domain_diversity");

    // Small file (10 structs, 3 domains) - baseline
    group.bench_function("small_file", |b| {
        let classifications = create_test_classifications(10, 3);
        b.iter(|| {
            DomainDiversityMetrics::from_struct_classifications(
                black_box(&classifications),
                black_box(false)
            )
        });
    });

    // Medium file (50 structs, 7 domains)
    group.bench_function("medium_file", |b| {
        let classifications = create_test_classifications(50, 7);
        b.iter(|| {
            DomainDiversityMetrics::from_struct_classifications(
                black_box(&classifications),
                black_box(false)
            )
        });
    });

    // Large file (200 structs, 15 domains)
    group.bench_function("large_file", |b| {
        let classifications = create_test_classifications(200, 15);
        b.iter(|| {
            DomainDiversityMetrics::from_struct_classifications(
                black_box(&classifications),
                black_box(false)
            )
        });
    });

    // Massive file (1000 structs, 50 domains) - stress test
    group.bench_function("massive_file", |b| {
        let classifications = create_test_classifications(1000, 50);
        b.iter(|| {
            DomainDiversityMetrics::from_struct_classifications(
                black_box(&classifications),
                black_box(false)
            )
        });
    });

    group.finish();
}

fn bench_entropy_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("entropy_calculation");

    for domain_count in [3, 10, 25, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(domain_count),
            domain_count,
            |b, &domain_count| {
                let classifications = create_test_classifications(100, domain_count);
                let distribution = group_structs_by_domain(&classifications);
                b.iter(|| {
                    calculate_domain_entropy(black_box(&distribution), black_box(100))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_domain_diversity_calculation, bench_entropy_calculation);
criterion_main!(benches);
```

**Performance Targets**:
- Small files (10 structs): <100μs
- Medium files (50 structs): <500μs
- Large files (200 structs): <2ms
- Massive files (1000 structs): <10ms
- **Overall overhead**: <5% of total analysis time

## Implementation Phases

### Phase 1: Core Pure Functions (MUST HAVE)
**Goal**: Implement type-safe, pure calculation functions

**Tasks**:
1. Create `src/organization/domain_diversity.rs` module
2. Implement `DiversityScore` newtype with validation
3. Implement `group_structs_by_domain()` pure function
4. Implement `calculate_domain_entropy()` pure function
5. Implement `determine_cross_domain_severity()` pure function
6. Write comprehensive unit tests for all pure functions
7. Add property-based tests for invariants

**Success Criteria**:
- [ ] All unit tests pass
- [ ] All property tests pass
- [ ] No clippy warnings
- [ ] Test coverage >90% for pure functions

### Phase 2: Integration & Output (SHOULD HAVE)
**Goal**: Integrate with existing analysis pipeline and format output

**Tasks**:
1. Implement `DomainDiversityMetrics::from_struct_classifications()`
2. Implement `DomainDiversityMetrics::format_for_output()`
3. Update `god_object_detector.rs` to calculate metrics
4. Update `formatter.rs` to display metrics
5. Fix "across 1 responsibilities" bug
6. Write integration tests

**Success Criteria**:
- [ ] config.rs analysis shows domain metrics
- [ ] Output format matches specification
- [ ] "Across 1 responsibilities" bug fixed
- [ ] Integration tests pass

### Phase 3: Performance Validation (MUST HAVE)
**Goal**: Ensure <5% overhead requirement is met

**Tasks**:
1. Add criterion benchmarks
2. Run benchmarks on various file sizes
3. Profile if overhead >5%
4. Optimize hot paths if needed
5. Document performance characteristics

**Success Criteria**:
- [ ] Benchmarks show <5% overhead
- [ ] Performance targets met (see above)
- [ ] No memory regressions on large files

### Phase 4: Polish & Documentation (NICE TO HAVE)
**Goal**: Complete documentation and classification evidence

**Tasks**:
1. Implement classification evidence formatting
2. Add verbose mode output
3. Update README.md with domain metrics section
4. Add inline documentation to all public APIs
5. Generate rustdoc for new module

**Success Criteria**:
- [ ] README.md updated
- [ ] All public APIs documented
- [ ] `cargo doc` builds without warnings
- [ ] Classification evidence available in verbose mode

## Rollout Strategy

### Feature Flag Approach (Recommended)

Initially ship behind a feature flag for validation:

```rust
#[cfg(feature = "domain-diversity-metrics")]
pub fn calculate_domain_metrics(...) -> Option<DomainDiversityMetrics> {
    // Implementation
}

#[cfg(not(feature = "domain-diversity-metrics"))]
pub fn calculate_domain_metrics(...) -> Option<DomainDiversityMetrics> {
    None
}
```

**Rollout Steps**:
1. Merge with feature flag disabled by default
2. Enable in CI for testing (1-2 weeks)
3. Enable by default in development builds
4. Collect feedback from users
5. Remove feature flag after validation (promote to stable)

### Migration Considerations

**Backward Compatibility**:
- Output format changes are additive only
- Existing output sections remain unchanged
- New domain metrics section appears after WHY statement

**JSON/Structured Output**:
If debtmap has structured output (JSON/YAML):
```json
{
  "file": "src/config.rs",
  "score": 100,
  "severity": "CRITICAL",
  "domain_diversity": {
    "total_structs": 30,
    "domain_count": 5,
    "diversity_score": 0.78,
    "severity": "CRITICAL",
    "domains": {
      "scoring": ["ScoringWeights", "RoleMultipliers"],
      "thresholds": ["ThresholdsConfig", "ValidationThresholds"],
      ...
    }
  }
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
- [ ] Performance benchmarks validate <5% overhead
- [ ] Type safety enforced with DiversityScore newtype
- [ ] Property tests validate invariants
- [ ] All edge cases handled gracefully

---

## Specification Improvements Summary

This specification has been enhanced with the following improvements:

### 1. **Functional Programming Patterns**
- Pure functions for all calculations (`group_structs_by_domain`, `calculate_domain_entropy`, `determine_cross_domain_severity`)
- Immutable data transformations using functional composition
- Separation of pure logic from I/O (formatting)
- Pattern matching for severity determination

### 2. **Type Safety**
- `DiversityScore` newtype enforces 0.0-1.0 bounds at compile/runtime
- `Result` types for fallible operations
- Strong typing prevents invalid states

### 3. **Error Handling**
- Graceful handling of edge cases (empty classifications, single domain, zero structs)
- Context-rich error messages using `anyhow::Context`
- No `.unwrap()` calls in production code paths

### 4. **Memory Efficiency**
- `Arc<str>` for shared string ownership instead of repeated cloning
- Efficient functional grouping using `fold`
- Clear memory ownership semantics

### 5. **Comprehensive Testing**
- Complete unit tests with edge case coverage
- Property-based tests for invariants (score bounds, single domain = 0.0, etc.)
- Integration tests for real-world scenarios
- Criterion benchmarks for performance validation
- Test helpers fully defined (`create_test_classifications`, `create_uniform_domain_classifications`)

### 6. **Performance Validation**
- Specific performance targets for different file sizes
- Criterion benchmark suite included
- <5% overhead requirement made testable
- Memory regression testing for large files

### 7. **Clear Module Organization**
- Public API clearly defined
- Private helpers documented
- Dependencies enumerated
- Responsibilities clearly separated

### 8. **Implementation Guidance**
- Four-phase implementation plan (MUST/SHOULD/NICE TO HAVE)
- Feature flag rollout strategy
- Migration considerations for backward compatibility
- Success criteria for each phase

### 9. **Documentation**
- Inline code documentation standards
- rustdoc requirements
- User-facing documentation updates
- JSON schema for structured output

This revised specification provides a complete, production-ready implementation guide that adheres to Rust and functional programming best practices while ensuring high quality, performance, and maintainability.
