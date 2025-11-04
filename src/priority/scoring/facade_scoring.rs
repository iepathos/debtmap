//! Module Facade Scoring (Spec 170)
//!
//! Provides score adjustment for god object detection based on module facade patterns.
//! Well-organized facade files with proper submodule separation receive significant
//! score reductions to avoid false positives.

use crate::analysis::module_structure::{ModuleFacadeInfo, OrganizationQuality};

/// Adjust god object score based on module facade detection (Spec 170)
///
/// Reduces scores for well-organized facade files to prevent false positives.
/// Monolithic files receive no reduction, maintaining accurate detection.
///
/// # Arguments
///
/// * `base_score` - The raw god object score before facade adjustment
/// * `facade_info` - Module facade detection results
/// * `method_count` - Total method/function count in the file
/// * `total_lines` - Total lines of code in the file
///
/// # Returns
///
/// Adjusted score after applying facade quality multipliers
///
/// # Examples
///
/// ```no_run
/// use debtmap::analysis::module_structure::{ModuleFacadeInfo, OrganizationQuality};
/// use debtmap::priority::scoring::facade_scoring::adjust_score_for_facade;
///
/// let facade_info = ModuleFacadeInfo {
///     is_facade: true,
///     submodule_count: 13,
///     path_declarations: vec![],
///     facade_score: 0.92,
///     organization_quality: OrganizationQuality::Excellent,
/// };
///
/// let base_score = 69.4;
/// let adjusted = adjust_score_for_facade(base_score, &facade_info, 91, 2257);
/// // Expected: ~1.7 (90% reduction for Excellent organization)
/// ```
pub fn adjust_score_for_facade(
    base_score: f64,
    facade_info: &ModuleFacadeInfo,
    method_count: usize,
    total_lines: usize,
) -> f64 {
    if !facade_info.is_facade {
        return base_score;
    }

    // Base multiplier from organization quality
    let quality_multiplier = match facade_info.organization_quality {
        OrganizationQuality::Excellent => 0.1,  // 90% reduction
        OrganizationQuality::Good => 0.3,       // 70% reduction
        OrganizationQuality::Poor => 0.6,       // 40% reduction
        OrganizationQuality::Monolithic => 1.0, // No reduction
    };

    // Bonus for high submodule count
    let submodule_bonus = match facade_info.submodule_count {
        0..=4 => 0.9, // Minimal bonus
        5..=9 => 0.7, // 30% additional reduction
        _ => 0.5,     // 50% additional reduction for ≥10 modules
    };

    // Check per-module metrics
    let avg_lines_per_module = total_lines / facade_info.submodule_count.max(1);
    let avg_methods_per_module = method_count / facade_info.submodule_count.max(1);

    let size_multiplier = if avg_lines_per_module < 300 && avg_methods_per_module < 15 {
        0.5 // Well-sized modules
    } else if avg_lines_per_module < 500 && avg_methods_per_module < 25 {
        0.7 // Moderate-sized modules
    } else {
        0.9 // Large modules might still need splitting
    };

    base_score * quality_multiplier * submodule_bonus * size_multiplier
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::module_structure::PathDeclaration;

    #[test]
    fn test_excellent_facade_score_reduction() {
        let facade_info = ModuleFacadeInfo {
            is_facade: true,
            submodule_count: 13,
            path_declarations: vec![],
            facade_score: 0.92,
            organization_quality: OrganizationQuality::Excellent,
        };

        let base_score = 69.4;
        let adjusted = adjust_score_for_facade(base_score, &facade_info, 91, 2257);

        // Should reduce by ~90%: 69.4 * 0.1 * 0.5 * 0.5 = ~1.7
        assert!(adjusted < 7.0, "Expected score < 7.0, got {}", adjusted);
        assert!(adjusted > 1.0, "Expected score > 1.0, got {}", adjusted);
    }

    #[test]
    fn test_monolithic_no_reduction() {
        let facade_info = ModuleFacadeInfo {
            is_facade: false,
            submodule_count: 0,
            path_declarations: vec![],
            facade_score: 0.05,
            organization_quality: OrganizationQuality::Monolithic,
        };

        let base_score = 69.4;
        let adjusted = adjust_score_for_facade(base_score, &facade_info, 91, 2257);

        assert_eq!(adjusted, base_score, "Monolithic score should not change");
    }

    #[test]
    fn test_good_organization_moderate_reduction() {
        let facade_info = ModuleFacadeInfo {
            is_facade: true,
            submodule_count: 6,
            path_declarations: vec![],
            facade_score: 0.65,
            organization_quality: OrganizationQuality::Good,
        };

        let base_score = 50.0;
        let adjusted = adjust_score_for_facade(base_score, &facade_info, 60, 1500);

        // Should reduce by ~70%: 50.0 * 0.3 * 0.7 * 0.5 = ~5.25
        assert!(adjusted < 15.0, "Expected score < 15.0, got {}", adjusted);
        assert!(adjusted > 3.0, "Expected score > 3.0, got {}", adjusted);
    }

    #[test]
    fn test_poor_organization_minimal_reduction() {
        let facade_info = ModuleFacadeInfo {
            is_facade: true,
            submodule_count: 3,
            path_declarations: vec![],
            facade_score: 0.55,
            organization_quality: OrganizationQuality::Poor,
        };

        let base_score = 40.0;
        let adjusted = adjust_score_for_facade(base_score, &facade_info, 80, 2000);

        // Should reduce by ~40%: 40.0 * 0.6 * 0.9 * 0.9 = ~19.4
        assert!(adjusted < 30.0, "Expected score < 30.0, got {}", adjusted);
        assert!(adjusted > 15.0, "Expected score > 15.0, got {}", adjusted);
    }

    #[test]
    fn test_large_submodules_reduced_benefit() {
        let facade_info = ModuleFacadeInfo {
            is_facade: true,
            submodule_count: 10,
            path_declarations: vec![],
            facade_score: 0.85,
            organization_quality: OrganizationQuality::Excellent,
        };

        // Large modules (600 lines each, 50 methods each)
        let base_score = 80.0;
        let adjusted = adjust_score_for_facade(base_score, &facade_info, 500, 6000);

        // Large modules get less reduction: 80.0 * 0.1 * 0.5 * 0.9 = ~3.6
        assert!(adjusted < 10.0, "Expected score < 10.0, got {}", adjusted);
        // But still some reduction
        assert!(adjusted < base_score * 0.2, "Should reduce by at least 80%");
    }

    #[test]
    fn test_small_well_organized_modules() {
        let facade_info = ModuleFacadeInfo {
            is_facade: true,
            submodule_count: 15,
            path_declarations: vec![],
            facade_score: 0.95,
            organization_quality: OrganizationQuality::Excellent,
        };

        // Small modules (150 lines each, 10 methods each)
        let base_score = 60.0;
        let adjusted = adjust_score_for_facade(base_score, &facade_info, 150, 2250);

        // Maximum reduction: 60.0 * 0.1 * 0.5 * 0.5 = 1.5
        assert!(adjusted < 5.0, "Expected score < 5.0, got {}", adjusted);
        assert!(adjusted < base_score * 0.1, "Should reduce by at least 90%");
    }

    #[test]
    fn test_facade_with_path_declarations() {
        let path_declarations = vec![
            PathDeclaration {
                module_name: "builder".to_string(),
                file_path: "executor/builder.rs".to_string(),
                line: 10,
            },
            PathDeclaration {
                module_name: "commands".to_string(),
                file_path: "executor/commands.rs".to_string(),
                line: 12,
            },
            PathDeclaration {
                module_name: "pure".to_string(),
                file_path: "executor/pure.rs".to_string(),
                line: 14,
            },
        ];

        let facade_info = ModuleFacadeInfo {
            is_facade: true,
            submodule_count: 3,
            path_declarations,
            facade_score: 0.88,
            organization_quality: OrganizationQuality::Good,
        };

        let base_score = 45.0;
        let adjusted = adjust_score_for_facade(base_score, &facade_info, 40, 800);

        // Good organization with path declarations
        assert!(adjusted < base_score * 0.5, "Should reduce by at least 50%");
    }
}
