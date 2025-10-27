/// Macro recommendation generation for boilerplate code patterns.
///
/// This module generates actionable recommendations for converting repetitive
/// boilerplate code into declarative macros, procedural macros, or code generation.
use super::boilerplate_detector::BoilerplatePattern;
use std::path::Path;

/// Engine for generating macro-specific recommendations
pub struct MacroRecommendationEngine;

impl MacroRecommendationEngine {
    /// Generate recommendation based on detected pattern
    pub fn generate_recommendation(pattern: &BoilerplatePattern, file_path: &Path) -> String {
        match pattern {
            BoilerplatePattern::TraitImplementation {
                trait_name,
                impl_count,
                shared_methods,
                method_uniformity,
            } => Self::generate_trait_macro_recommendation(
                trait_name,
                *impl_count,
                shared_methods,
                *method_uniformity,
                file_path,
            ),
            BoilerplatePattern::BuilderPattern { builder_count } => {
                Self::generate_builder_recommendation(*builder_count)
            }
            BoilerplatePattern::TestBoilerplate {
                test_count,
                shared_structure,
            } => Self::generate_test_boilerplate_recommendation(*test_count, shared_structure),
        }
    }

    /// Generate recommendation for trait implementation boilerplate
    fn generate_trait_macro_recommendation(
        trait_name: &str,
        impl_count: usize,
        shared_methods: &[String],
        method_uniformity: f64,
        _file_path: &Path,
    ) -> String {
        let current_lines = impl_count * 75; // Estimated lines per impl
        let macro_lines = impl_count * 8; // Estimated lines with macro
        let reduction_pct = ((current_lines - macro_lines) as f64 / current_lines as f64) * 100.0;

        let shared_methods_list = if shared_methods.len() <= 5 {
            shared_methods.join(", ")
        } else {
            format!(
                "{}, and {} more",
                shared_methods[..5].join(", "),
                shared_methods.len() - 5
            )
        };

        format!(
            "BOILERPLATE DETECTED: {} implementations of {} trait ({}% method uniformity).\n\
            \n\
            This file contains repetitive trait implementations that should be \n\
            macro-ified or code-generated.\n\
            \n\
            RECOMMENDED APPROACH:\n\
            1. Create a declarative macro to generate {} implementations\n\
            2. Replace {} trait impl blocks with macro invocations\n\
            3. Expected reduction: {} lines → ~{} lines ({:.0}% reduction)\n\
            \n\
            SHARED METHODS ACROSS IMPLEMENTATIONS:\n\
            {}\n\
            \n\
            EXAMPLE TRANSFORMATION:\n\
            \n\
            // Before: ~75 lines per implementation\n\
            struct MyFlag;\n\
            impl {} for MyFlag {{\n\
                fn name_long(&self) -> &'static str {{ \"my-flag\" }}\n\
                fn is_switch(&self) -> bool {{ true }}\n\
                fn doc_category(&self) -> &'static str {{ \"general\" }}\n\
                // ... 70 more boilerplate lines\n\
            }}\n\
            \n\
            // After: ~8 lines per implementation\n\
            flag! {{\n\
                MyFlag {{\n\
                    long: \"my-flag\",\n\
                    is_switch: true,\n\
                    category: \"general\",\n\
                    // ... declarative config\n\
                }}\n\
            }}\n\
            \n\
            ALTERNATIVES:\n\
            - Build-time code generation from schema file (JSON/TOML)\n\
            - Use existing derive macro crates if applicable\n\
            - Consider procedural macros for complex transformations\n\
            \n\
            CONFIDENCE: {:.0}%\n\
            \n\
            This is NOT a god object requiring module splitting. The high method/line count\n\
            is due to declarative boilerplate, not complexity. Focus on reducing repetition\n\
            through macros rather than splitting into multiple files.",
            impl_count,
            trait_name,
            (method_uniformity * 100.0) as usize,
            trait_name,
            impl_count,
            current_lines,
            macro_lines,
            reduction_pct,
            shared_methods_list,
            trait_name,
            (method_uniformity * 100.0)
        )
    }

    /// Generate recommendation for builder pattern boilerplate
    fn generate_builder_recommendation(builder_count: usize) -> String {
        format!(
            "BUILDER PATTERN BOILERPLATE DETECTED: {} builder structs\n\
            \n\
            Consider using existing builder libraries:\n\
            - `bon` crate for declarative builders\n\
            - `typed-builder` for compile-time checked builders\n\
            - `derive_builder` for macro-based generation\n\
            \n\
            This reduces boilerplate while maintaining type safety.",
            builder_count
        )
    }

    /// Generate recommendation for test boilerplate
    fn generate_test_boilerplate_recommendation(
        test_count: usize,
        shared_structure: &str,
    ) -> String {
        format!(
            "TEST BOILERPLATE DETECTED: {} tests with shared structure\n\
            \n\
            Consider parameterized testing:\n\
            - Use `rstest` for parameterized tests\n\
            - Extract common setup to test fixtures\n\
            - Use table-driven test patterns\n\
            \n\
            Shared structure: {}",
            test_count, shared_structure
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_trait_macro_recommendation_format() {
        let pattern = BoilerplatePattern::TraitImplementation {
            trait_name: "Flag".to_string(),
            impl_count: 104,
            shared_methods: vec![
                "name_long".to_string(),
                "is_switch".to_string(),
                "doc_category".to_string(),
            ],
            method_uniformity: 0.95,
        };

        let path = PathBuf::from("src/flags/defs.rs");
        let recommendation = MacroRecommendationEngine::generate_recommendation(&pattern, &path);

        assert!(recommendation.contains("104 implementations"));
        assert!(recommendation.contains("Flag trait"));
        assert!(recommendation.contains("95% method uniformity"));
        assert!(recommendation.contains("name_long"));
        assert!(recommendation.contains("BOILERPLATE DETECTED"));
        assert!(recommendation.contains("89% reduction")); // 104 * 75 = 7800, 104 * 8 = 832, (7800-832)/7800 = 89%
    }

    #[test]
    fn test_builder_recommendation() {
        let pattern = BoilerplatePattern::BuilderPattern { builder_count: 5 };
        let path = PathBuf::from("src/config.rs");
        let recommendation = MacroRecommendationEngine::generate_recommendation(&pattern, &path);

        assert!(recommendation.contains("BUILDER PATTERN"));
        assert!(recommendation.contains("bon"));
        assert!(recommendation.contains("typed-builder"));
    }

    #[test]
    fn test_test_boilerplate_recommendation() {
        let pattern = BoilerplatePattern::TestBoilerplate {
            test_count: 50,
            shared_structure: "setup -> action -> assert".to_string(),
        };
        let path = PathBuf::from("tests/integration.rs");
        let recommendation = MacroRecommendationEngine::generate_recommendation(&pattern, &path);

        assert!(recommendation.contains("50 tests"));
        assert!(recommendation.contains("rstest"));
        assert!(recommendation.contains("setup -> action -> assert"));
    }

    #[test]
    fn test_line_reduction_calculation() {
        let pattern = BoilerplatePattern::TraitImplementation {
            trait_name: "Flag".to_string(),
            impl_count: 100,
            shared_methods: vec![],
            method_uniformity: 0.9,
        };

        let path = PathBuf::from("src/flags.rs");
        let recommendation = MacroRecommendationEngine::generate_recommendation(&pattern, &path);

        // Should show reduction from ~7500 lines to ~800 lines
        assert!(recommendation.contains("7500 lines"));
        assert!(recommendation.contains("800 lines"));
        assert!(recommendation.contains("89% reduction"));
    }

    #[test]
    fn test_shared_methods_truncation() {
        let pattern = BoilerplatePattern::TraitImplementation {
            trait_name: "ComplexTrait".to_string(),
            impl_count: 50,
            shared_methods: vec![
                "method1".to_string(),
                "method2".to_string(),
                "method3".to_string(),
                "method4".to_string(),
                "method5".to_string(),
                "method6".to_string(),
                "method7".to_string(),
            ],
            method_uniformity: 0.85,
        };

        let path = PathBuf::from("src/trait_impls.rs");
        let recommendation = MacroRecommendationEngine::generate_recommendation(&pattern, &path);

        // Should truncate list and show "and 2 more"
        assert!(recommendation.contains("and 2 more"));
    }
}
