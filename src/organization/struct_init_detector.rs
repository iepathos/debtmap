use super::{MaintainabilityImpact, OrganizationAntiPattern, OrganizationDetector};
use crate::common::{LocationConfidence, SourceLocation};
use crate::organization::struct_initialization::StructInitPatternDetector;
use syn::File;

pub struct StructInitOrganizationDetector {
    pattern_detector: StructInitPatternDetector,
}

impl StructInitOrganizationDetector {
    pub fn new() -> Self {
        Self {
            pattern_detector: StructInitPatternDetector::new(),
        }
    }
}

impl Default for StructInitOrganizationDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl OrganizationDetector for StructInitOrganizationDetector {
    fn detect_anti_patterns(&self, file: &File) -> Vec<OrganizationAntiPattern> {
        let file_content = String::new(); // We'll need actual content for span analysis

        // For now, return empty vec since we need file content
        // In real integration, this will be provided via context
        if let Some(pattern) = self.pattern_detector.detect(file, &file_content) {
            let confidence = self.pattern_detector.confidence(&pattern);
            let field_complexity = self
                .pattern_detector
                .calculate_init_complexity_score(&pattern);
            let recommendation = self.pattern_detector.generate_recommendation(&pattern);

            // Only report if confidence is high enough
            if confidence >= 0.60 {
                let location = SourceLocation {
                    line: 1, // We'd extract from pattern in real impl
                    column: Some(0),
                    end_line: None,
                    end_column: None,
                    confidence: LocationConfidence::Approximate,
                };

                vec![OrganizationAntiPattern::StructInitialization {
                    function_name: pattern.struct_name.clone(),
                    struct_name: pattern.struct_name,
                    field_count: pattern.field_count,
                    cyclomatic_complexity: pattern.cyclomatic_complexity,
                    field_based_complexity: field_complexity,
                    confidence,
                    recommendation,
                    location,
                }]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    fn detector_name(&self) -> &'static str {
        "StructInitializationDetector"
    }

    fn estimate_maintainability_impact(
        &self,
        pattern: &OrganizationAntiPattern,
    ) -> MaintainabilityImpact {
        if let OrganizationAntiPattern::StructInitialization {
            field_count,
            cyclomatic_complexity,
            confidence,
            ..
        } = pattern
        {
            // High confidence + high complexity = higher impact
            if *confidence > 0.80 && *cyclomatic_complexity > 30 {
                MaintainabilityImpact::High
            } else if *field_count > 40 {
                MaintainabilityImpact::Medium
            } else {
                MaintainabilityImpact::Low
            }
        } else {
            MaintainabilityImpact::Low
        }
    }
}
