//! File-level coupling metrics and classification (spec 201, 269)
//!
//! Provides coupling classification and instability calculation for files
//! based on afferent (incoming) and efferent (outgoing) dependencies.
//!
//! Spec 269 adds architectural awareness using the Stable Dependencies Principle:
//! - Stable modules (low instability) with high callers are intentional architecture
//! - Unstable modules (high instability) with high callers are potential debt

use super::format::round_ratio;
use serde::{Deserialize, Serialize};

/// File-level dependency metrics (spec 201)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDependencies {
    /// Number of files that depend on this file
    pub afferent_coupling: usize,
    /// Number of files this file depends on
    pub efferent_coupling: usize,
    /// Instability metric (0.0 = stable, 1.0 = unstable)
    pub instability: f64,
    /// Total coupling (Ca + Ce)
    pub total_coupling: usize,
    /// Files that depend on this file (top N)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_dependents: Vec<String>,
    /// Files this file depends on (top N)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_dependencies: Vec<String>,
    /// Classification based on coupling characteristics
    pub coupling_classification: CouplingClassification,
}

/// Classification of file-level coupling characteristics (spec 201, 269)
///
/// Spec 269 enhances this enum with architectural awareness based on the
/// Stable Dependencies Principle from Clean Architecture. Classifications
/// now consider test coverage and instability to identify intentional
/// stable foundations vs actual architectural debt.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CouplingClassification {
    // Architecture-aware classifications (spec 269)
    /// Low instability + high test caller ratio (>70%) - well-tested foundation
    WellTestedCore,
    /// Low instability + high production callers - stable foundation by design
    StableFoundation,
    /// High instability + many production callers - actual architectural debt
    UnstableHighCoupling,

    // Existing classifications (spec 201)
    /// Low instability, high afferent coupling - core module others depend on
    StableCore,
    /// Balanced coupling - typical utility module
    UtilityModule,
    /// Central connector with balanced instability (many callers and callees)
    ArchitecturalHub,
    /// High instability, low afferent coupling - peripheral module
    LeafModule,
    /// Very low total coupling - may be dead code or standalone
    Isolated,
    /// High total coupling - may need refactoring (legacy, prefer UnstableHighCoupling)
    HighlyCoupled,
}

impl CouplingClassification {
    /// Returns true if this classification indicates architectural concern (potential debt).
    ///
    /// These are modules that violate the Stable Dependencies Principle:
    /// unstable modules with many dependents, or architectural hubs that may
    /// be bottlenecks.
    pub fn is_architectural_concern(&self) -> bool {
        matches!(
            self,
            Self::UnstableHighCoupling | Self::ArchitecturalHub | Self::HighlyCoupled
        )
    }

    /// Returns true if this classification indicates stable-by-design architecture.
    ///
    /// These modules follow the Stable Dependencies Principle and should NOT
    /// be flagged as debt just because they have many callers.
    pub fn is_stable_by_design(&self) -> bool {
        matches!(
            self,
            Self::WellTestedCore | Self::StableFoundation | Self::StableCore
        )
    }

    /// Returns a score multiplier based on architectural classification (spec 269).
    ///
    /// - Stable-by-design modules get reduced scores (less debt priority)
    /// - Architectural concerns get neutral or increased scores
    ///
    /// Multipliers:
    /// - WellTestedCore: 0.2 (80% reduction - excellent architecture with high test coverage)
    /// - StableFoundation: 0.5 (50% reduction - good stable design)
    /// - StableCore: 0.6 (40% reduction - stable module)
    /// - LeafModule: 0.8 (20% reduction - normal dependency)
    /// - Isolated: 0.9 (10% reduction - low risk)
    /// - UtilityModule: 1.0 (no change - neutral)
    /// - ArchitecturalHub: 1.0 (no change - needs review but may be valid)
    /// - HighlyCoupled: 1.2 (20% increase - legacy warning)
    /// - UnstableHighCoupling: 1.5 (50% increase - actual debt)
    pub fn score_multiplier(&self) -> f64 {
        match self {
            Self::WellTestedCore => 0.2,
            Self::StableFoundation => 0.5,
            Self::StableCore => 0.6,
            Self::LeafModule => 0.8,
            Self::Isolated => 0.9,
            Self::UtilityModule => 1.0,
            Self::ArchitecturalHub => 1.0,
            Self::HighlyCoupled => 1.2,
            Self::UnstableHighCoupling => 1.5,
        }
    }
}

impl std::fmt::Display for CouplingClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CouplingClassification::WellTestedCore => write!(f, "Well-Tested Core"),
            CouplingClassification::StableFoundation => write!(f, "Stable Foundation"),
            CouplingClassification::UnstableHighCoupling => write!(f, "Unstable High Coupling"),
            CouplingClassification::StableCore => write!(f, "Stable Core"),
            CouplingClassification::UtilityModule => write!(f, "Utility Module"),
            CouplingClassification::ArchitecturalHub => write!(f, "Architectural Hub"),
            CouplingClassification::LeafModule => write!(f, "Leaf Module"),
            CouplingClassification::Isolated => write!(f, "Isolated"),
            CouplingClassification::HighlyCoupled => write!(f, "Highly Coupled"),
        }
    }
}

/// Classify coupling based on metrics (spec 201)
pub fn classify_coupling(afferent: usize, efferent: usize) -> CouplingClassification {
    let total = afferent + efferent;
    let instability = if total > 0 {
        efferent as f64 / total as f64
    } else {
        0.0
    };

    // Highly coupled threshold
    if total > 15 {
        return CouplingClassification::HighlyCoupled;
    }

    // Isolated module
    if total < 3 {
        return CouplingClassification::Isolated;
    }

    // Stable core: low instability, reasonable afferent coupling
    if instability < 0.3 && afferent >= 3 {
        return CouplingClassification::StableCore;
    }

    // Leaf module: high instability, low afferent coupling
    if instability > 0.7 && afferent <= 2 {
        return CouplingClassification::LeafModule;
    }

    // Default: utility module with balanced coupling
    CouplingClassification::UtilityModule
}

/// Calculate instability metric from coupling values
pub fn calculate_instability(afferent: usize, efferent: usize) -> f64 {
    let total = afferent + efferent;
    if total > 0 {
        efferent as f64 / total as f64
    } else {
        0.0
    }
}

/// Classify coupling pattern with architectural awareness (spec 269).
///
/// This function uses the Stable Dependencies Principle to classify modules:
/// - Stable modules (low instability) with many callers are intentional foundations
/// - Unstable modules (high instability) with many callers are potential debt
///
/// # Arguments
///
/// * `instability` - I = Ce/(Ca+Ce), range [0.0, 1.0]. Lower = more stable.
/// * `production_caller_count` - Number of production code callers (from spec 267)
/// * `test_caller_count` - Number of test code callers (from spec 267)
/// * `callee_count` - Number of functions/modules this depends on
///
/// # Returns
///
/// A `CouplingClassification` that reflects the architectural role of this module.
///
/// # Classification Logic
///
/// 1. WellTestedCore: Low instability (< 0.3) + high test ratio (> 0.7) + sufficient callers (> 5)
/// 2. StableFoundation: Low instability (< 0.3) + many production callers (> 10)
/// 3. StableCore: Low instability (< 0.3) + moderate callers (> 5)
/// 4. UnstableHighCoupling: High instability (> 0.7) + many production callers (> 5) - DEBT
/// 5. ArchitecturalHub: Balanced instability (0.3-0.7) + high coupling (> 10 callers)
/// 6. LeafModule: Few callers (< 3) + many callees (> 5)
/// 7. Isolated: Minimal coupling (< 3 callers and < 3 callees)
/// 8. Default: LeafModule
pub fn classify_coupling_pattern(
    instability: f64,
    production_caller_count: usize,
    test_caller_count: usize,
    callee_count: usize,
) -> CouplingClassification {
    let total_callers = production_caller_count + test_caller_count;
    let test_ratio = if total_callers > 0 {
        test_caller_count as f64 / total_callers as f64
    } else {
        0.0
    };

    // Classification decision tree based on Clean Architecture's Stable Dependencies Principle
    match (
        instability,
        total_callers,
        test_ratio,
        production_caller_count,
        callee_count,
    ) {
        // Well-tested core: stable + mostly test callers
        // Use <= 0.35 to handle floating point precision (0.3023 displays as "0.30")
        (i, c, t, _, _) if i <= 0.35 && c > 5 && t > 0.7 => CouplingClassification::WellTestedCore,

        // Stable foundation: stable + many production callers
        (i, _, _, p, _) if i <= 0.35 && p > 10 => CouplingClassification::StableFoundation,

        // Stable core: stable + moderate callers
        (i, c, _, _, _) if i <= 0.35 && c > 5 => CouplingClassification::StableCore,

        // Unstable high coupling: unstable + many production callers (DEBT)
        (i, _, _, p, _) if i > 0.7 && p > 5 => CouplingClassification::UnstableHighCoupling,

        // Architectural hub: balanced instability + high coupling
        (i, c, _, _, _) if i > 0.3 && i < 0.7 && c > 10 => CouplingClassification::ArchitecturalHub,

        // Leaf module: depends on many, few depend on it
        (_, c, _, _, callees) if c < 3 && callees > 5 => CouplingClassification::LeafModule,

        // Isolated: minimal coupling
        (_, c, _, _, callees) if c < 3 && callees < 3 => CouplingClassification::Isolated,

        // Default to leaf module for unclassified cases
        _ => CouplingClassification::LeafModule,
    }
}

/// Calculate architectural dependency factor with classification (spec 269).
///
/// This combines the dependency factor calculation with architectural awareness,
/// returning both the adjusted factor and the classification.
///
/// # Arguments
///
/// * `production_upstream_count` - Production callers (from spec 267)
/// * `test_upstream_count` - Test callers (from spec 267)
/// * `downstream_count` - Callees
///
/// # Returns
///
/// Tuple of (adjusted_factor, classification) where:
/// - adjusted_factor: Base dependency factor * score_multiplier
/// - classification: The architectural classification
pub fn calculate_architectural_dependency_factor(
    production_upstream_count: usize,
    test_upstream_count: usize,
    downstream_count: usize,
) -> (f64, CouplingClassification) {
    let incoming = production_upstream_count + test_upstream_count;
    let instability = calculate_instability(incoming, downstream_count);

    let classification = classify_coupling_pattern(
        instability,
        production_upstream_count,
        test_upstream_count,
        downstream_count,
    );

    // Base factor from production callers only (spec 267)
    // Uses a logarithmic scale: more callers = higher factor, but diminishing returns
    let base_factor = if production_upstream_count == 0 {
        0.0
    } else {
        // Scale: 1 caller = 1.0, 10 callers = 3.3, 100 callers = 6.6
        (1.0 + production_upstream_count as f64).ln() / 1.5
    };

    // Apply architectural multiplier
    let adjusted_factor = base_factor * classification.score_multiplier();

    (adjusted_factor.min(10.0), classification)
}

/// Build FileDependencies from file metrics (spec 201)
pub fn build_file_dependencies(
    metrics: &crate::priority::FileDebtMetrics,
) -> Option<FileDependencies> {
    // Only include if there's some coupling data
    let has_coupling_data = metrics.afferent_coupling > 0
        || metrics.efferent_coupling > 0
        || !metrics.dependents.is_empty()
        || !metrics.dependencies_list.is_empty();

    if !has_coupling_data {
        return None;
    }

    let afferent = metrics.afferent_coupling;
    let efferent = metrics.efferent_coupling;

    Some(FileDependencies {
        afferent_coupling: afferent,
        efferent_coupling: efferent,
        instability: round_ratio(metrics.instability),
        total_coupling: afferent + efferent,
        top_dependents: metrics.dependents.iter().take(5).cloned().collect(),
        top_dependencies: metrics.dependencies_list.iter().take(5).cloned().collect(),
        coupling_classification: classify_coupling(afferent, efferent),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_instability_balanced() {
        // Equal afferent and efferent should give 0.5 instability
        let instability = calculate_instability(5, 5);
        assert!((instability - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_calculate_instability_stable() {
        // High afferent, no efferent should give 0.0 instability
        let instability = calculate_instability(10, 0);
        assert!((instability - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_instability_unstable() {
        // No afferent, high efferent should give 1.0 instability
        let instability = calculate_instability(0, 10);
        assert!((instability - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_instability_zero_coupling() {
        // Zero coupling should give 0.0 instability
        let instability = calculate_instability(0, 0);
        assert!((instability - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_classify_coupling_stable_core() {
        // Low instability, high afferent = stable core
        let classification = classify_coupling(8, 2); // instability = 0.2
        assert_eq!(classification, CouplingClassification::StableCore);
    }

    #[test]
    fn test_classify_coupling_leaf_module() {
        // High instability, low afferent = leaf module
        let classification = classify_coupling(1, 8); // instability = 0.89
        assert_eq!(classification, CouplingClassification::LeafModule);
    }

    #[test]
    fn test_classify_coupling_utility_module() {
        // Balanced coupling = utility module
        let classification = classify_coupling(5, 5);
        assert_eq!(classification, CouplingClassification::UtilityModule);
    }

    #[test]
    fn test_classify_coupling_isolated() {
        // Very low total coupling = isolated
        let classification = classify_coupling(1, 1);
        assert_eq!(classification, CouplingClassification::Isolated);
    }

    #[test]
    fn test_classify_coupling_highly_coupled() {
        // High total coupling > 15 = highly coupled
        let classification = classify_coupling(10, 10);
        assert_eq!(classification, CouplingClassification::HighlyCoupled);
    }

    #[test]
    fn test_coupling_classification_display() {
        assert_eq!(
            format!("{}", CouplingClassification::StableCore),
            "Stable Core"
        );
        assert_eq!(
            format!("{}", CouplingClassification::LeafModule),
            "Leaf Module"
        );
        assert_eq!(format!("{}", CouplingClassification::Isolated), "Isolated");
        assert_eq!(
            format!("{}", CouplingClassification::HighlyCoupled),
            "Highly Coupled"
        );
    }

    #[test]
    fn test_file_dependencies_serialization() {
        let deps = FileDependencies {
            afferent_coupling: 5,
            efferent_coupling: 3,
            instability: 0.375,
            total_coupling: 8,
            top_dependents: vec!["main.rs".to_string(), "lib.rs".to_string()],
            top_dependencies: vec!["std".to_string()],
            coupling_classification: CouplingClassification::UtilityModule,
        };

        let json = serde_json::to_string(&deps).unwrap();
        assert!(json.contains("\"afferent_coupling\":5"));
        assert!(json.contains("\"efferent_coupling\":3"));
        assert!(json.contains("\"instability\":0.375"));
        assert!(json.contains("\"total_coupling\":8"));
        assert!(json.contains("\"top_dependents\":[\"main.rs\",\"lib.rs\"]"));
        assert!(json.contains("\"top_dependencies\":[\"std\"]"));
        assert!(json.contains("\"coupling_classification\":\"utility_module\""));
    }

    #[test]
    fn test_file_dependencies_empty_lists_not_serialized() {
        let deps = FileDependencies {
            afferent_coupling: 0,
            efferent_coupling: 0,
            instability: 0.0,
            total_coupling: 0,
            top_dependents: vec![],
            top_dependencies: vec![],
            coupling_classification: CouplingClassification::Isolated,
        };

        let json = serde_json::to_string(&deps).unwrap();
        // Empty vectors should be skipped
        assert!(!json.contains("\"top_dependents\""));
        assert!(!json.contains("\"top_dependencies\""));
    }

    // Spec 269: Architecture-aware classification tests

    #[test]
    fn test_well_tested_core_classification() {
        // Low instability + high test ratio + sufficient callers
        let classification = classify_coupling_pattern(
            0.2, // Low instability
            5,   // 5 production callers
            85,  // 85 test callers (94% test ratio)
            10,  // 10 callees
        );
        assert_eq!(classification, CouplingClassification::WellTestedCore);
    }

    #[test]
    fn test_stable_foundation_classification() {
        // Low instability + many production callers
        let classification = classify_coupling_pattern(
            0.2, // Low instability
            15,  // 15 production callers (> 10)
            5,   // 5 test callers
            10,  // 10 callees
        );
        assert_eq!(classification, CouplingClassification::StableFoundation);
    }

    #[test]
    fn test_stable_core_classification_with_architecture() {
        // Low instability + moderate callers (not enough for foundation)
        let classification = classify_coupling_pattern(
            0.25, // Low instability
            6,    // 6 production callers (not > 10)
            2,    // 2 test callers (low test ratio)
            5,    // 5 callees
        );
        assert_eq!(classification, CouplingClassification::StableCore);
    }

    #[test]
    fn test_unstable_high_coupling_classification() {
        // High instability + many production callers = DEBT
        let classification = classify_coupling_pattern(
            0.8, // High instability
            15,  // 15 production callers
            5,   // 5 test callers
            80,  // Many callees (causing high instability)
        );
        assert_eq!(classification, CouplingClassification::UnstableHighCoupling);
    }

    #[test]
    fn test_architectural_hub_classification() {
        // Balanced instability + high coupling
        let classification = classify_coupling_pattern(
            0.5, // Balanced instability
            8,   // 8 production callers
            5,   // 5 test callers (total 13 > 10)
            13,  // 13 callees
        );
        assert_eq!(classification, CouplingClassification::ArchitecturalHub);
    }

    #[test]
    fn test_leaf_module_classification_with_architecture() {
        // Few callers + many callees
        let classification = classify_coupling_pattern(
            0.9, // High instability (few incoming, many outgoing)
            1,   // 1 production caller
            1,   // 1 test caller (total 2 < 3)
            10,  // 10 callees (> 5)
        );
        assert_eq!(classification, CouplingClassification::LeafModule);
    }

    #[test]
    fn test_isolated_classification_with_architecture() {
        // Minimal coupling in both directions
        let classification = classify_coupling_pattern(
            0.5, // Balanced (but doesn't matter)
            1,   // 1 production caller
            0,   // 0 test callers (total 1 < 3)
            2,   // 2 callees (< 3)
        );
        assert_eq!(classification, CouplingClassification::Isolated);
    }

    #[test]
    fn test_score_multipliers() {
        // Stable modules should reduce score
        assert!(CouplingClassification::WellTestedCore.score_multiplier() < 0.5);
        assert!(CouplingClassification::StableFoundation.score_multiplier() < 1.0);
        assert!(CouplingClassification::StableCore.score_multiplier() < 1.0);

        // Debt indicators should increase score
        assert!(CouplingClassification::UnstableHighCoupling.score_multiplier() > 1.0);
        assert!(CouplingClassification::HighlyCoupled.score_multiplier() > 1.0);

        // Neutral modules should be around 1.0
        assert!((CouplingClassification::UtilityModule.score_multiplier() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_is_stable_by_design() {
        assert!(CouplingClassification::WellTestedCore.is_stable_by_design());
        assert!(CouplingClassification::StableFoundation.is_stable_by_design());
        assert!(CouplingClassification::StableCore.is_stable_by_design());

        assert!(!CouplingClassification::UnstableHighCoupling.is_stable_by_design());
        assert!(!CouplingClassification::LeafModule.is_stable_by_design());
        assert!(!CouplingClassification::ArchitecturalHub.is_stable_by_design());
    }

    #[test]
    fn test_is_architectural_concern() {
        assert!(CouplingClassification::UnstableHighCoupling.is_architectural_concern());
        assert!(CouplingClassification::ArchitecturalHub.is_architectural_concern());
        assert!(CouplingClassification::HighlyCoupled.is_architectural_concern());

        assert!(!CouplingClassification::WellTestedCore.is_architectural_concern());
        assert!(!CouplingClassification::StableFoundation.is_architectural_concern());
        assert!(!CouplingClassification::LeafModule.is_architectural_concern());
    }

    #[test]
    fn test_architectural_dependency_factor_well_tested() {
        // Well-tested core should have reduced factor
        let (factor, classification) = calculate_architectural_dependency_factor(
            5,  // 5 production callers
            85, // 85 test callers
            10, // 10 callees
        );

        assert_eq!(classification, CouplingClassification::WellTestedCore);
        // Base factor for 5 callers ≈ ln(6)/1.5 ≈ 1.2, multiplied by 0.3 ≈ 0.36
        assert!(factor < 1.0, "Well-tested core should have reduced factor");
    }

    #[test]
    fn test_architectural_dependency_factor_unstable() {
        // Unstable high coupling should have increased factor
        let (factor, classification) = calculate_architectural_dependency_factor(
            10, // 10 production callers
            2,  // 2 test callers
            50, // 50 callees (high instability)
        );

        assert_eq!(classification, CouplingClassification::UnstableHighCoupling);
        // Base factor for 10 callers ≈ ln(11)/1.5 ≈ 1.6, multiplied by 1.5 ≈ 2.4
        assert!(
            factor > 2.0,
            "Unstable high coupling should have increased factor"
        );
    }

    #[test]
    fn test_coupling_classification_new_display() {
        // Test new variants display correctly
        assert_eq!(
            format!("{}", CouplingClassification::WellTestedCore),
            "Well-Tested Core"
        );
        assert_eq!(
            format!("{}", CouplingClassification::StableFoundation),
            "Stable Foundation"
        );
        assert_eq!(
            format!("{}", CouplingClassification::UnstableHighCoupling),
            "Unstable High Coupling"
        );
        assert_eq!(
            format!("{}", CouplingClassification::ArchitecturalHub),
            "Architectural Hub"
        );
    }

    #[test]
    fn test_overflow_rs_scenario() {
        // Simulate the overflow.rs scenario from the spec:
        // Instability 0.26, Blast Radius 121 (5 prod + 85 test), Coupling: Stable Core
        let instability = 0.26;
        let production_callers = 5;
        let test_callers = 85;
        let callees = 35; // Results in instability ~0.26

        let classification =
            classify_coupling_pattern(instability, production_callers, test_callers, callees);

        // Should be WellTestedCore because:
        // - instability < 0.3 ✓
        // - total callers (90) > 5 ✓
        // - test ratio (85/90 = 0.94) > 0.7 ✓
        assert_eq!(classification, CouplingClassification::WellTestedCore);
        assert!(classification.is_stable_by_design());
        assert!(classification.score_multiplier() < 0.5);
    }
}
