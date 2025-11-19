use crate::organization::GodObjectAnalysis;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DetailedRecommendation {
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub action_items: Vec<String>,
    pub estimated_effort: EffortEstimate,
    pub impact: ImpactAssessment,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffortEstimate {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub struct ImpactAssessment {
    pub complexity_reduction: i32,
    pub maintainability_improvement: i32,
    pub testability_improvement: i32,
    pub risk_reduction: i32,
}

pub fn generate_god_object_recommendation(
    analysis: &GodObjectAnalysis,
    path: &Path,
) -> DetailedRecommendation {
    let file_name = path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file");

    let mut action_items = vec![
        "Break into smaller, focused modules:".to_string(),
    ];

    for split in &analysis.recommended_splits {
        let mut split_desc = format!(
            "  - {} ({} methods, ~{} lines)",
            split.suggested_name,
            split.methods_to_move.len(),
            split.estimated_lines
        );

        // Add behavior category if available
        if let Some(ref category) = split.behavior_category {
            split_desc.push_str(&format!(" [{}]", category));
        }

        action_items.push(split_desc);

        // Add representative methods if available
        if !split.representative_methods.is_empty() {
            let methods_preview = split.representative_methods
                .iter()
                .take(3)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            action_items.push(format!("    Methods: {}, ...", methods_preview));
        }

        // Add fields needed if available
        if !split.fields_needed.is_empty() {
            let fields_preview = split.fields_needed
                .iter()
                .take(5)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            action_items.push(format!("    Fields: {}", fields_preview));
        }

        // Add trait suggestion if available
        if let Some(ref trait_suggestion) = split.trait_suggestion {
            let first_line = trait_suggestion.lines().next().unwrap_or("");
            action_items.push(format!("    Suggested trait: {}", first_line));
        }
    }

    action_items.push("Apply SOLID principles, especially Single Responsibility".to_string());
    action_items.push("Create interfaces/traits for better abstraction".to_string());
    action_items.push("Add comprehensive tests before refactoring".to_string());

    DetailedRecommendation {
        severity: Severity::Critical,

        title: if analysis.field_count > 5 && analysis.method_count > 20 {
            format!(
                "[CRITICAL] GOD OBJECT: {} ({} methods, {} fields, {} responsibilities)",
                file_name,
                analysis.method_count,
                analysis.field_count,
                analysis.responsibility_count
            )
        } else {
            format!(
                "[CRITICAL] GOD MODULE: {} ({} module functions, {} responsibilities)",
                file_name,
                analysis.method_count,
                analysis.responsibility_count
            )
        },

        description: if analysis.field_count > 5 && analysis.method_count > 20 {
            format!(
                "This struct has grown too large and handles too many responsibilities. \
                 With {} methods, {} fields, and {} lines of code, it's become difficult to maintain, \
                 test, and understand. This is a high priority for refactoring.",
                analysis.method_count,
                analysis.field_count,
                analysis.lines_of_code
            )
        } else {
            format!(
                "This module contains {} module functions across {} responsibilities. \
                 With {} lines of code and {} total complexity, it's become difficult to navigate \
                 and maintain. Consider splitting into focused sub-modules.",
                analysis.method_count,
                analysis.responsibility_count,
                analysis.lines_of_code,
                analysis.complexity_sum
            )
        },
        
        action_items,
        
        estimated_effort: EffortEstimate::High,
        
        impact: ImpactAssessment {
            complexity_reduction: (analysis.complexity_sum / 2) as i32,
            maintainability_improvement: 80,
            testability_improvement: 70,
            risk_reduction: 90,
        },
    }
}

pub fn format_god_object_display(
    analysis: &GodObjectAnalysis,
    path: &Path,
    score: f64,
    rank: usize,
) -> String {
    let mut output = String::new();

    // Determine if this is a god class or god module
    let is_god_class = analysis.field_count > 5 && analysis.method_count > 20;
    let label = if is_god_class { "GOD OBJECT" } else { "GOD MODULE" };
    let metric_label = if is_god_class { "Methods" } else { "Module Functions" };

    output.push_str(&format!(
        "#{} SCORE: {:.1} [[CRITICAL] {}]\n",
        rank, score, label
    ));

    output.push_str(&format!(
        "   └─ {}\n",
        path.display()
    ));

    output.push_str(&format!("\n   {} METRICS:\n", label));
    output.push_str(&format!("   ├─ {}: {} (max: {})\n", metric_label, analysis.method_count, if is_god_class { 20 } else { 50 }));
    if is_god_class {
        output.push_str(&format!("   ├─ Fields: {} (max: 15)\n", analysis.field_count));
    }
    output.push_str(&format!("   ├─ Responsibilities: {} (max: {})\n", analysis.responsibility_count, if is_god_class { 3 } else { 4 }));
    output.push_str(&format!("   ├─ Lines: {} (max: 1,000)\n", analysis.lines_of_code));
    output.push_str(&format!("   └─ Total Complexity: {}\n", analysis.complexity_sum));
    
    if !analysis.recommended_splits.is_empty() {
        output.push_str("\n   [INFO] RECOMMENDED REFACTORING:\n");
        output.push_str("   Split into focused modules:\n");

        for (i, split) in analysis.recommended_splits.iter().enumerate() {
            let is_last = i == analysis.recommended_splits.len() - 1;
            let prefix = if is_last { "└─" } else { "├─" };

            // Module header with behavior category
            let mut header = format!(
                "   {} {} (~{} lines, {} methods)",
                prefix,
                split.suggested_name,
                split.estimated_lines,
                split.methods_to_move.len()
            );
            if let Some(ref category) = split.behavior_category {
                header.push_str(&format!(" [{}]", category));
            }
            output.push_str(&format!("{}\n", header));

            let sub_prefix = if is_last { "  " } else { "│ " };

            // Responsibility
            output.push_str(&format!(
                "   {}  └─ {}\n",
                sub_prefix, split.responsibility
            ));

            // Representative methods
            if !split.representative_methods.is_empty() {
                let methods = split.representative_methods
                    .iter()
                    .take(3)
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                output.push_str(&format!(
                    "   {}     Methods: {}{}\n",
                    sub_prefix,
                    methods,
                    if split.representative_methods.len() > 3 {
                        ", ..."
                    } else {
                        ""
                    }
                ));
            }

            // Fields needed
            if !split.fields_needed.is_empty() {
                let fields = split.fields_needed
                    .iter()
                    .take(5)
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                output.push_str(&format!("   {}     Fields: {}\n", sub_prefix, fields));
            }

            // Trait suggestion (first line only)
            if let Some(ref trait_suggestion) = split.trait_suggestion {
                if let Some(first_line) = trait_suggestion.lines().next() {
                    output.push_str(&format!("   {}     {}\n", sub_prefix, first_line));
                }
            }
        }
    }
    
    output.push_str(&format!(
        "\n   [PERF] IMPACT: -{} complexity, +80% maintainability, +70% testability\n",
        analysis.complexity_sum / 2
    ));
    
    output
}
