//! File-level coupling metrics and classification (spec 201)
//!
//! Provides coupling classification and instability calculation for files
//! based on afferent (incoming) and efferent (outgoing) dependencies.

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

/// Classification of file-level coupling characteristics (spec 201)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CouplingClassification {
    /// Low instability, high afferent coupling - core module others depend on
    StableCore,
    /// Balanced coupling - typical utility module
    UtilityModule,
    /// High instability, low afferent coupling - peripheral module
    LeafModule,
    /// Very low total coupling - may be dead code or standalone
    Isolated,
    /// High total coupling - may need refactoring
    HighlyCoupled,
}

impl std::fmt::Display for CouplingClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CouplingClassification::StableCore => write!(f, "Stable Core"),
            CouplingClassification::UtilityModule => write!(f, "Utility Module"),
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
}
