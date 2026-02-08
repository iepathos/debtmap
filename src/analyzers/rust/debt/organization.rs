//! Organization pattern analysis
//!
//! Detects organizational anti-patterns in Rust code.

use crate::core::{DebtItem, DebtType, Priority};
use crate::organization::{
    FeatureEnvyDetector, GodObjectDetector, MagicValueDetector, MaintainabilityImpact,
    OrganizationAntiPattern, OrganizationDetector, ParameterAnalyzer, PrimitiveObsessionDetector,
    StructInitOrganizationDetector,
};
use std::path::Path;

/// Analyze organization patterns in a file
pub fn analyze_organization_patterns(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    let detectors: Vec<Box<dyn OrganizationDetector>> = vec![
        Box::new(GodObjectDetector::new()),
        Box::new(MagicValueDetector::new()),
        Box::new(ParameterAnalyzer::new()),
        Box::new(FeatureEnvyDetector::new()),
        Box::new(PrimitiveObsessionDetector::new()),
        Box::new(StructInitOrganizationDetector::new()),
    ];

    let mut organization_items = Vec::new();

    for detector in detectors {
        let anti_patterns = detector.detect_anti_patterns(file);

        for pattern in anti_patterns {
            let impact = detector.estimate_maintainability_impact(&pattern);
            let debt_item = convert_organization_pattern_to_debt_item(pattern, impact, path);
            organization_items.push(debt_item);
        }
    }

    organization_items
}

/// Convert impact to priority
fn impact_to_priority(impact: MaintainabilityImpact) -> Priority {
    match impact {
        MaintainabilityImpact::Critical => Priority::Critical,
        MaintainabilityImpact::High => Priority::High,
        MaintainabilityImpact::Medium => Priority::Medium,
        MaintainabilityImpact::Low => Priority::Low,
    }
}

/// Extract message and context from pattern
pub fn pattern_to_message_context(pattern: &OrganizationAntiPattern) -> (String, Option<String>) {
    match pattern {
        OrganizationAntiPattern::GodObject {
            type_name,
            method_count,
            field_count,
            suggested_split,
            ..
        } => (
            format!(
                "God object '{}' with {} methods and {} fields",
                type_name, method_count, field_count
            ),
            Some(format!(
                "Consider splitting into: {}",
                suggested_split
                    .iter()
                    .map(|g| g.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        ),
        OrganizationAntiPattern::MagicValue {
            value,
            occurrence_count,
            suggested_constant_name,
            ..
        } => (
            format!("Magic value '{}' appears {} times", value, occurrence_count),
            Some(format!(
                "Extract constant: const {} = {};",
                suggested_constant_name, value
            )),
        ),
        OrganizationAntiPattern::LongParameterList {
            function_name,
            parameter_count,
            suggested_refactoring,
            ..
        } => (
            format!(
                "Function '{}' has {} parameters",
                function_name, parameter_count
            ),
            Some(format!("Consider: {:?}", suggested_refactoring)),
        ),
        OrganizationAntiPattern::FeatureEnvy {
            method_name,
            envied_type,
            external_calls,
            internal_calls,
            ..
        } => (
            format!(
                "Method '{}' makes {} external calls vs {} internal calls",
                method_name, external_calls, internal_calls
            ),
            Some(format!("Consider moving to '{}'", envied_type)),
        ),
        OrganizationAntiPattern::PrimitiveObsession {
            primitive_type,
            usage_context,
            suggested_domain_type,
            ..
        } => (
            format!(
                "Primitive obsession: '{}' used for {:?}",
                primitive_type, usage_context
            ),
            Some(format!("Consider domain type: {}", suggested_domain_type)),
        ),
        OrganizationAntiPattern::DataClump {
            parameter_group,
            suggested_struct_name,
            ..
        } => (
            format!(
                "Data clump with {} parameters",
                parameter_group.parameters.len()
            ),
            Some(format!("Extract struct: {}", suggested_struct_name)),
        ),
        OrganizationAntiPattern::StructInitialization {
            function_name,
            field_count,
            cyclomatic_complexity,
            field_based_complexity,
            confidence,
            recommendation,
            ..
        } => (
            format!(
                "Struct initialization pattern in '{}' - {} fields, cyclomatic: {}, field complexity: {:.1}, confidence: {:.0}%",
                function_name, field_count, cyclomatic_complexity, field_based_complexity, confidence * 100.0
            ),
            Some(format!(
                "{} (Use field-based complexity {:.1} instead of cyclomatic {})",
                recommendation, field_based_complexity, cyclomatic_complexity
            )),
        ),
    }
}

fn convert_organization_pattern_to_debt_item(
    pattern: OrganizationAntiPattern,
    impact: MaintainabilityImpact,
    path: &Path,
) -> DebtItem {
    let location = pattern.primary_location().clone();
    let line = location.line;

    let priority = impact_to_priority(impact);
    let (message, context) = pattern_to_message_context(&pattern);

    DebtItem {
        id: format!("organization-{}-{}", path.display(), line),
        debt_type: DebtType::CodeOrganization {
            issue_type: Some(pattern.pattern_type().to_string()),
        },
        priority,
        file: path.to_path_buf(),
        line,
        column: location.column,
        message,
        context,
    }
}
