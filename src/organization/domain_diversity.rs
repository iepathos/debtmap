/// Domain diversity metrics for struct-based split recommendations (Spec 152).
///
/// This module provides pure functional calculation of domain diversity metrics
/// for files with multiple structs that span different semantic domains.
/// It calculates entropy-based diversity scores and cross-domain mixing severity.
///
/// All core functions are pure (no side effects) and use functional composition.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type-safe diversity score (0.0 = homogeneous, 1.0 = maximum diversity).
///
/// Represents Shannon entropy normalized to 0.0-1.0 range.
/// - 0.0: All structs in single domain
/// - 1.0: Maximum diversity (evenly distributed across all domains)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DiversityScore(f64);

impl DiversityScore {
    /// Create a new diversity score with validation.
    ///
    /// # Errors
    /// Returns error if value is not in [0.0, 1.0] range or is NaN.
    pub fn new(value: f64) -> Result<Self> {
        if !value.is_finite() {
            anyhow::bail!(
                "Diversity score must be finite, got {}",
                if value.is_nan() { "NaN" } else { "infinity" }
            );
        }
        if !(0.0..=1.0).contains(&value) {
            anyhow::bail!("Diversity score must be between 0.0 and 1.0, got {}", value);
        }
        Ok(DiversityScore(value))
    }

    /// Create a diversity score of 0.0 (homogeneous).
    pub fn zero() -> Self {
        DiversityScore(0.0)
    }

    /// Get the underlying f64 value.
    pub fn as_f64(self) -> f64 {
        self.0
    }
}

/// Cross-domain mixing severity levels per Spec 140.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossDomainSeverity {
    /// CRITICAL: God object + 3+ domains, OR 15+ structs + 5+ domains
    Critical,
    /// HIGH: 10+ structs + 4+ domains
    High,
    /// MEDIUM: 8+ structs + 3+ domains
    Medium,
    /// LOW: 5+ structs + 3+ domains (informational)
    Low,
}

impl CrossDomainSeverity {
    /// Get string representation for output.
    pub fn as_str(self) -> &'static str {
        match self {
            CrossDomainSeverity::Critical => "CRITICAL",
            CrossDomainSeverity::High => "HIGH",
            CrossDomainSeverity::Medium => "MEDIUM",
            CrossDomainSeverity::Low => "LOW",
        }
    }
}

/// Domain diversity metrics for a file with multiple structs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainDiversityMetrics {
    pub total_structs: usize,
    pub domain_count: usize,
    pub domain_distribution: HashMap<String, Vec<String>>,
    pub diversity_score: DiversityScore,
    pub severity: CrossDomainSeverity,
    pub is_god_object: bool,
}

impl DomainDiversityMetrics {
    /// Pure functional constructor from struct classifications.
    ///
    /// Calculates domain diversity metrics from struct domain classifications.
    /// All computation is pure with no side effects.
    ///
    /// # Arguments
    /// * `classifications` - List of struct domain classifications
    /// * `is_god_object` - Whether this file is classified as a god object
    ///
    /// # Returns
    /// Domain diversity metrics or error if calculation fails
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

        let severity = determine_cross_domain_severity(total_structs, domain_count, is_god_object);

        Ok(DomainDiversityMetrics {
            total_structs,
            domain_count,
            domain_distribution,
            diversity_score,
            severity,
            is_god_object,
        })
    }

    /// Get the largest domain (most structs).
    pub fn largest_domain(&self) -> Option<(&String, usize)> {
        self.domain_distribution
            .iter()
            .map(|(domain, structs)| (domain, structs.len()))
            .max_by_key(|(_, count)| *count)
    }

    /// Get the smallest domain (fewest structs).
    pub fn smallest_domain(&self) -> Option<(&String, usize)> {
        self.domain_distribution
            .iter()
            .map(|(domain, structs)| (domain, structs.len()))
            .min_by_key(|(_, count)| *count)
    }

    /// Format domain diversity metrics for output.
    pub fn format_for_output(&self) -> String {
        let mut output = String::new();

        // Header with severity
        output.push_str(&format!(
            "\nDOMAIN DIVERSITY ANALYSIS (Spec 140):\n\
             Severity: {} - {} structs across {} domains\n\n",
            self.severity.as_str(),
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
            let examples: Vec<&String> = structs.iter().take(3).collect();
            output.push_str(&format!(
                "    Examples: {}{}",
                examples
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<&str>>()
                    .join(", "),
                if structs.len() > 3 {
                    format!(", ... +{} more", structs.len() - 3)
                } else {
                    String::new()
                }
            ));
            output.push('\n');
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

    /// Format severity justification for output.
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
                    self.total_structs, self.domain_count
                )
            }
            CrossDomainSeverity::High => {
                format!(
                    "  Reason: {} structs across {} domains (Spec 140: HIGH threshold)\n\
                     → HIGH PRIORITY: Significant organizational debt",
                    self.total_structs, self.domain_count
                )
            }
            CrossDomainSeverity::Medium => {
                format!(
                    "  Reason: {} structs across {} domains (Spec 140: MEDIUM threshold)\n\
                     → PROACTIVE: Good time to organize before it grows",
                    self.total_structs, self.domain_count
                )
            }
            CrossDomainSeverity::Low => {
                format!(
                    "  Reason: {} structs across {} domains (Spec 140: LOW threshold)\n\
                     → INFORMATIONAL: Minor organizational improvement opportunity",
                    self.total_structs, self.domain_count
                )
            }
        }
    }
}

/// Struct domain classification with evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDomainClassification {
    pub struct_name: String,
    pub domain: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub alternatives: Vec<(String, f64)>,
}

impl StructDomainClassification {
    /// Create a simple classification with just name and domain.
    pub fn simple(struct_name: String, domain: String) -> Self {
        StructDomainClassification {
            struct_name,
            domain,
            confidence: 1.0,
            evidence: vec![],
            alternatives: vec![],
        }
    }

    /// Format evidence for output.
    pub fn format_evidence(&self) -> String {
        let mut output = format!(
            "{} → {} domain ({:.0}% confidence)\n",
            self.struct_name,
            self.domain,
            self.confidence * 100.0
        );

        if !self.evidence.is_empty() {
            output.push_str(&format!("  Evidence: {}\n", self.evidence.join(", ")));
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

// ============================================================================
// Pure helper functions
// ============================================================================

/// Pure function: Group structs by domain using functional composition.
///
/// Groups struct names by their domain classification.
fn group_structs_by_domain(
    classifications: &[StructDomainClassification],
) -> HashMap<String, Vec<String>> {
    classifications
        .iter()
        .fold(HashMap::new(), |mut acc, classification| {
            acc.entry(classification.domain.clone())
                .or_default()
                .push(classification.struct_name.clone());
            acc
        })
}

/// Pure function: Calculate Shannon entropy normalized to 0.0-1.0.
///
/// Shannon entropy: H = -Σ p_i * log2(p_i)
/// Normalized diversity: H / log2(n) where n = number of domains
///
/// # Arguments
/// * `distribution` - Map of domain to list of struct names
/// * `total` - Total number of structs
///
/// # Returns
/// Diversity score in [0.0, 1.0] range
fn calculate_domain_entropy(
    distribution: &HashMap<String, Vec<String>>,
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
        (entropy / max_entropy).clamp(0.0, 1.0) // Clamp to handle floating point precision errors
    } else {
        0.0
    };

    DiversityScore::new(normalized).context("Entropy calculation produced invalid diversity score")
}

/// Pure function: Determine severity based on Spec 140 thresholds.
///
/// Severity levels (from Spec 140):
/// - CRITICAL: God object + 3+ domains, OR 15+ structs + 5+ domains
/// - HIGH: 10+ structs + 4+ domains
/// - MEDIUM: 8+ structs + 3+ domains
/// - LOW: All other cases
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // Test helper: Create evenly distributed classifications
    fn create_test_classifications(
        struct_count: usize,
        domain_count: usize,
    ) -> Vec<StructDomainClassification> {
        let domains: Vec<String> = (0..domain_count).map(|i| format!("domain_{}", i)).collect();

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
    fn create_uniform_domain_classifications(
        struct_count: usize,
    ) -> Vec<StructDomainClassification> {
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
        assert_eq!(metrics.domain_count, 2); // scoring, thresholds
        assert!(metrics.diversity_score.as_f64() > 0.0);
        assert!(metrics.diversity_score.as_f64() < 1.0); // Not maximum diversity
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
        assert_eq!(metrics.diversity_score.as_f64(), 0.0); // Single domain = 0.0 diversity
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
        let classifications = create_test_classifications(10, 3); // 10 structs, 3 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(
            &classifications,
            true, // is_god_object
        )
        .expect("Failed to create metrics");

        assert_eq!(metrics.severity, CrossDomainSeverity::Critical);
    }

    #[test]
    fn critical_severity_for_massive_mixing() {
        let classifications = create_test_classifications(20, 5); // 20 structs, 5 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(
            &classifications,
            false, // not god object, but still critical
        )
        .expect("Failed to create metrics");

        assert_eq!(metrics.severity, CrossDomainSeverity::Critical);
    }

    #[test]
    fn high_severity() {
        let classifications = create_test_classifications(12, 4); // 12 structs, 4 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
            .expect("Failed to create metrics");

        assert_eq!(metrics.severity, CrossDomainSeverity::High);
    }

    #[test]
    fn medium_severity() {
        let classifications = create_test_classifications(8, 3); // 8 structs, 3 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
            .expect("Failed to create metrics");

        assert_eq!(metrics.severity, CrossDomainSeverity::Medium);
    }

    #[test]
    fn low_severity() {
        let classifications = create_test_classifications(4, 2); // 4 structs, 2 domains

        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
            .expect("Failed to create metrics");

        assert_eq!(metrics.severity, CrossDomainSeverity::Low);
    }

    #[test]
    fn largest_domain_detection() {
        let classifications = vec![
            StructDomainClassification::simple("A".into(), "domain1".into()),
            StructDomainClassification::simple("B".into(), "domain1".into()),
            StructDomainClassification::simple("C".into(), "domain1".into()),
            StructDomainClassification::simple("D".into(), "domain2".into()),
        ];

        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
            .expect("Failed to create metrics");

        let (domain, count) = metrics
            .largest_domain()
            .expect("Should have largest domain");
        assert_eq!(domain.as_str(), "domain1");
        assert_eq!(count, 3);
    }

    #[test]
    fn smallest_domain_detection() {
        let classifications = vec![
            StructDomainClassification::simple("A".into(), "domain1".into()),
            StructDomainClassification::simple("B".into(), "domain1".into()),
            StructDomainClassification::simple("C".into(), "domain2".into()),
        ];

        let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
            .expect("Failed to create metrics");

        let (domain, count) = metrics
            .smallest_domain()
            .expect("Should have smallest domain");
        assert_eq!(domain.as_str(), "domain2");
        assert_eq!(count, 1);
    }

    // Property-based tests using proptest
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn diversity_score_always_bounded(
                struct_count in 1usize..100,
                domain_count in 1usize..20
            ) {
                let classifications = create_test_classifications(struct_count, domain_count.min(struct_count));
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
                    create_test_classifications(struct_count, domain_count.min(struct_count))
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
                let classifications = create_test_classifications(struct_count, domain_count.min(struct_count));
                let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, true)
                    .expect("Failed to create metrics");

                prop_assert_eq!(metrics.severity, CrossDomainSeverity::Critical);
            }

            #[test]
            fn domain_count_never_exceeds_struct_count(
                struct_count in 1usize..100,
                domain_count in 1usize..20
            ) {
                let classifications = create_test_classifications(struct_count, domain_count);
                let metrics = DomainDiversityMetrics::from_struct_classifications(&classifications, false)
                    .expect("Failed to create metrics");

                prop_assert!(metrics.domain_count <= metrics.total_structs);
            }
        }
    }
}
