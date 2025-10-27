/// Boilerplate pattern detection for identifying repetitive code that should be
/// macro-ified or code-generated rather than split into modules.
///
/// This detector distinguishes between:
/// - **Complex code**: High cyclomatic complexity, diverse logic → split into modules
/// - **Boilerplate code**: Low complexity, repetitive patterns → macro-ify or codegen
///
/// # Example
///
/// ```rust
/// use debtmap::organization::BoilerplateDetector;
/// use syn::parse_str;
///
/// let detector = BoilerplateDetector::default();
/// let content = std::fs::read_to_string("src/file.rs")?;
/// let ast = parse_str(&content)?;
/// let analysis = detector.detect(Path::new("src/file.rs"), &ast);
///
/// if analysis.is_boilerplate {
///     println!("Confidence: {:.0}%", analysis.confidence * 100.0);
///     println!("{}", analysis.recommendation);
/// }
/// ```
use super::trait_pattern_analyzer::{TraitPatternAnalyzer, TraitPatternMetrics};
use serde::{Deserialize, Serialize};
use std::path::Path;
use syn;

/// Core boilerplate detection configuration and logic
#[derive(Debug, Clone)]
pub struct BoilerplateDetector {
    /// Minimum number of impl blocks to consider
    pub min_impl_blocks: usize,
    /// Minimum percentage of shared methods across implementations (0.0-1.0)
    pub method_uniformity_threshold: f64,
    /// Maximum average complexity for boilerplate classification
    pub max_avg_complexity: f64,
    /// Minimum confidence to report as boilerplate (0.0-1.0)
    pub confidence_threshold: f64,
}

impl Default for BoilerplateDetector {
    fn default() -> Self {
        Self {
            min_impl_blocks: 20,
            method_uniformity_threshold: 0.7,
            max_avg_complexity: 2.0,
            confidence_threshold: 0.7,
        }
    }
}

impl BoilerplateDetector {
    /// Create detector from configuration
    pub fn from_config(config: &BoilerplateDetectionConfig) -> Self {
        Self {
            min_impl_blocks: config.min_impl_blocks,
            method_uniformity_threshold: config.method_uniformity_threshold,
            max_avg_complexity: config.max_avg_complexity,
            confidence_threshold: config.confidence_threshold,
        }
    }

    /// Detect boilerplate patterns in a file
    ///
    /// Returns analysis with confidence score and pattern type if detected.
    pub fn detect(&self, path: &Path, ast: &syn::File) -> BoilerplateAnalysis {
        // Analyze trait patterns
        let trait_metrics = TraitPatternAnalyzer::analyze_file(ast);

        // Check if trait implementation pattern qualifies as boilerplate
        if trait_metrics.impl_block_count >= self.min_impl_blocks {
            let confidence = self.calculate_trait_boilerplate_confidence(&trait_metrics);

            if confidence >= self.confidence_threshold {
                let signals = self.extract_detection_signals(&trait_metrics);
                let pattern = self.classify_pattern(&trait_metrics);
                let recommendation =
                    super::macro_recommendations::MacroRecommendationEngine::generate_recommendation(
                        &pattern,
                        path,
                    );

                return BoilerplateAnalysis {
                    is_boilerplate: true,
                    confidence,
                    pattern_type: Some(pattern),
                    signals,
                    recommendation,
                };
            }
        }

        // No boilerplate detected
        BoilerplateAnalysis {
            is_boilerplate: false,
            confidence: 0.0,
            pattern_type: None,
            signals: vec![],
            recommendation: String::new(),
        }
    }

    /// Calculate confidence score for trait boilerplate pattern
    fn calculate_trait_boilerplate_confidence(&self, metrics: &TraitPatternMetrics) -> f64 {
        let mut score = 0.0;
        let max_score = 100.0;

        // Signal 1: Many trait implementations (30% weight)
        if metrics.impl_block_count >= self.min_impl_blocks {
            let normalized = (metrics.impl_block_count as f64 / 100.0).min(1.0);
            score += 30.0 * normalized;
        }

        // Signal 2: Method uniformity (25% weight)
        if metrics.method_uniformity >= self.method_uniformity_threshold {
            score += 25.0 * metrics.method_uniformity;
        }

        // Signal 3: Low complexity (20% weight)
        if metrics.avg_method_complexity < self.max_avg_complexity {
            let inverse_complexity =
                1.0 - (metrics.avg_method_complexity / self.max_avg_complexity);
            score += 20.0 * inverse_complexity;
        }

        // Signal 4: Low complexity variance (15% weight)
        if metrics.complexity_variance < 2.0 {
            let normalized = 1.0 - (metrics.complexity_variance / 10.0).min(1.0);
            score += 15.0 * normalized;
        }

        // Signal 5: Single dominant trait (10% weight)
        if let Some((_, count)) = &metrics.most_common_trait {
            let ratio = *count as f64 / metrics.impl_block_count as f64;
            if ratio > 0.8 {
                score += 10.0 * ratio;
            }
        }

        (score / max_score).min(1.0)
    }

    /// Extract detection signals from metrics
    fn extract_detection_signals(&self, metrics: &TraitPatternMetrics) -> Vec<DetectionSignal> {
        let mut signals = Vec::new();

        if metrics.impl_block_count >= self.min_impl_blocks {
            signals.push(DetectionSignal::HighImplCount(metrics.impl_block_count));
        }

        if metrics.method_uniformity >= self.method_uniformity_threshold {
            signals.push(DetectionSignal::HighMethodUniformity(
                metrics.method_uniformity,
            ));
        }

        if metrics.avg_method_complexity < self.max_avg_complexity {
            signals.push(DetectionSignal::LowAvgComplexity(
                metrics.avg_method_complexity,
            ));
        }

        if metrics.complexity_variance < 2.0 {
            signals.push(DetectionSignal::LowComplexityVariance(
                metrics.complexity_variance,
            ));
        }

        signals
    }

    /// Classify the specific boilerplate pattern
    fn classify_pattern(&self, metrics: &TraitPatternMetrics) -> BoilerplatePattern {
        BoilerplatePattern::TraitImplementation {
            trait_name: metrics
                .most_common_trait
                .as_ref()
                .map(|(name, _)| name.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            impl_count: metrics.impl_block_count,
            shared_methods: metrics
                .shared_methods
                .iter()
                .map(|(name, _)| name.clone())
                .collect(),
            method_uniformity: metrics.method_uniformity,
        }
    }
}

/// Result of boilerplate detection analysis
#[derive(Debug, Clone)]
pub struct BoilerplateAnalysis {
    /// Whether boilerplate pattern was detected
    pub is_boilerplate: bool,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Type of boilerplate pattern detected
    pub pattern_type: Option<BoilerplatePattern>,
    /// Detection signals that contributed to classification
    pub signals: Vec<DetectionSignal>,
    /// Generated recommendation text
    pub recommendation: String,
}

/// Types of boilerplate patterns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BoilerplatePattern {
    /// Many trait implementations with similar structure
    TraitImplementation {
        trait_name: String,
        impl_count: usize,
        shared_methods: Vec<String>,
        method_uniformity: f64,
    },
    /// Builder pattern with many setter methods
    BuilderPattern { builder_count: usize },
    /// Repetitive test functions
    TestBoilerplate {
        test_count: usize,
        shared_structure: String,
    },
}

/// Detection signals indicating boilerplate
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DetectionSignal {
    HighImplCount(usize),
    HighMethodUniformity(f64),
    LowAvgComplexity(f64),
    HighStructDensity(usize),
    LowComplexityVariance(f64),
}

/// Configuration for boilerplate detection
#[derive(Debug, Clone)]
pub struct BoilerplateDetectionConfig {
    pub enabled: bool,
    pub min_impl_blocks: usize,
    pub method_uniformity_threshold: f64,
    pub max_avg_complexity: f64,
    pub confidence_threshold: f64,
    pub detect_trait_impls: bool,
    pub detect_builders: bool,
    pub detect_test_boilerplate: bool,
}

impl Default for BoilerplateDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_impl_blocks: 20,
            method_uniformity_threshold: 0.7,
            max_avg_complexity: 2.0,
            confidence_threshold: 0.7,
            detect_trait_impls: true,
            detect_builders: true,
            detect_test_boilerplate: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_detector_thresholds() {
        let detector = BoilerplateDetector::default();
        assert_eq!(detector.min_impl_blocks, 20);
        assert_eq!(detector.method_uniformity_threshold, 0.7);
        assert_eq!(detector.max_avg_complexity, 2.0);
        assert_eq!(detector.confidence_threshold, 0.7);
    }

    #[test]
    fn test_from_config() {
        let config = BoilerplateDetectionConfig {
            min_impl_blocks: 50,
            confidence_threshold: 0.9,
            ..Default::default()
        };
        let detector = BoilerplateDetector::from_config(&config);
        assert_eq!(detector.min_impl_blocks, 50);
        assert_eq!(detector.confidence_threshold, 0.9);
    }

    #[test]
    fn test_calculate_confidence_high_score() {
        let detector = BoilerplateDetector::default();
        let metrics = TraitPatternMetrics {
            impl_block_count: 100,
            unique_traits: std::collections::HashSet::from(["Flag".to_string()]),
            most_common_trait: Some(("Flag".to_string(), 100)),
            method_uniformity: 0.95,
            shared_methods: vec![
                ("name_long".to_string(), 1.0),
                ("is_switch".to_string(), 1.0),
            ],
            avg_method_complexity: 1.2,
            complexity_variance: 0.5,
            avg_method_lines: 3.0,
        };

        let confidence = detector.calculate_trait_boilerplate_confidence(&metrics);
        assert!(
            confidence > 0.85,
            "Confidence should be high for clear boilerplate"
        );
    }

    #[test]
    fn test_calculate_confidence_low_score() {
        let detector = BoilerplateDetector::default();
        let metrics = TraitPatternMetrics {
            impl_block_count: 5, // Below threshold
            unique_traits: std::collections::HashSet::from(["Trait1".to_string()]),
            most_common_trait: Some(("Trait1".to_string(), 5)),
            method_uniformity: 0.4, // Low uniformity
            shared_methods: vec![],
            avg_method_complexity: 8.0, // High complexity
            complexity_variance: 5.0,   // High variance
            avg_method_lines: 15.0,
        };

        let confidence = detector.calculate_trait_boilerplate_confidence(&metrics);
        assert!(
            confidence < 0.5,
            "Confidence should be low for non-boilerplate"
        );
    }
}
